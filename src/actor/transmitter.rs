use std::future::ready;
use std::io::{self, ErrorKind};
use std::time::{Duration, Instant};

use log::{debug, info, warn};
use tokio::io::AsyncWrite;
use tokio::select;
use tokio::sync::mpsc::error::TrySendError;
use tokio::sync::mpsc::{Receiver, Sender};

use self::buffer::Buffer;
use self::transmission::Transmission;
use crate::actor::message::Message;
use crate::actor::receiver::Received;
use crate::actor::timer::{timeout, wait_until};
use crate::frame::{Ack, Data, Frame, Nak};
use crate::protocol::{Mask, next_sequence_number};
use crate::validate::Validate;
use crate::{Payload, SEQ_MASK, T_RSTACK_MAX_MILLIS, T_RX_ACK_MAX_MILLIS, TX_K};

mod buffer;
mod transmission;

/// Time to wait for each reset acknowledgement.
const T_RSTACK_MAX: Duration = Duration::from_millis(T_RSTACK_MAX_MILLIS);

/// Time to wait for acknowledgement of the oldest outstanding DATA frame.
const T_RX_ACK_MAX: Duration = Duration::from_millis(T_RX_ACK_MAX_MILLIS);

/// Refresh nRdy before the NCP's default one-second callback pause expires.
const NOT_READY_REFRESH: Duration = Duration::from_millis(500);

/// One initial reset attempt plus five retries.
const RESET_ATTEMPTS: usize = 6;

const INITIAL_SEQUENCE_NUMBER: u8 = 0;

const RECEIVE_TRANSPORT_CLOSED: &str = "ASHv2 receive transport closed";
const RESPONSE_CHANNEL_CLOSED: &str = "ASHv2 response channel closed";
const WINDOW_EXCEEDED: &str = "ASHv2 transmit window exceeded";

/// Sends frames and owns both directions of the ASH link state.
#[derive(Debug)]
pub struct Transmitter<T> {
    buffer: Buffer<T>,
    requests: Receiver<Message>,
    frames: Receiver<Received>,
    response: Sender<Payload>,
    transmissions: heapless::Vec<Transmission, { TX_K.get() }>,
    retries_remaining: usize,
    frame_number: u8,
    ack_number: u8,
    rejecting: bool,
    not_ready_until: Option<Instant>,
}

impl<T> Transmitter<T> {
    /// Creates a transmitter with independent queues for application writes and serial input.
    pub const fn new(
        writer: T,
        requests: Receiver<Message>,
        frames: Receiver<Received>,
        response: Sender<Payload>,
    ) -> Self {
        Self {
            buffer: Buffer::new(writer),
            requests,
            frames,
            response,
            transmissions: heapless::Vec::new(),
            retries_remaining: 0,
            frame_number: INITIAL_SEQUENCE_NUMBER,
            ack_number: INITIAL_SEQUENCE_NUMBER,
            rejecting: false,
            not_ready_until: None,
        }
    }
}

impl<T> Transmitter<T>
where
    T: AsyncWrite + Unpin,
{
    /// Drives the link until normal shutdown or a terminal error.
    pub async fn run(mut self) {
        let result = self.run_link().await;
        self.frames.close();

        if let Err(error) = result {
            warn!("ASHv2 link stopped: {error}");
            self.requests.close();
            let reason = error.to_string();
            let failure = || Err(io::Error::new(error.kind(), reason.clone()));
            for transmission in self.transmissions.drain(..) {
                transmission.complete(failure());
            }
            while let Some(request) = self.requests.recv().await {
                let _ = request.response_tx.send(failure());
            }
        }
    }

    /// Establishes the link and drives it until shutdown or failure.
    async fn run_link(&mut self) -> io::Result<()> {
        self.connect().await?;

        loop {
            let requests_drained = self.requests.is_closed() && self.requests.is_empty();
            if requests_drained && self.transmissions.is_empty() {
                break;
            }
            self.handle_next_event(requests_drained).await?;
        }
        Ok(())
    }

    /// Waits for and handles one event on the established link.
    async fn handle_next_event(&mut self, requests_drained: bool) -> io::Result<()> {
        let retrying = self.retries_remaining > 0;
        let ack_deadline = self
            .transmissions
            .first()
            .map(|oldest| oldest.sent_at() + T_RX_ACK_MAX);
        let can_accept_request = !requests_drained && !self.transmissions.is_full();

        select! {
            biased;
            () = wait_until(ack_deadline), if !retrying => {
                debug!("ASHv2 DATA acknowledgement timed out");
                self.start_retransmission();
            }
            () = wait_until(self.not_ready_until) => self.send_readiness_ack().await?,
            available = async { self.response.reserve().await.map(drop) },
                if self.not_ready_until.is_some() =>
            {
                available.map_err(|_| io::Error::new(ErrorKind::BrokenPipe, RESPONSE_CHANNEL_CLOSED))?;
                self.send_readiness_ack().await?;
            }
            received = self.frames.recv() => {
                let frame = received.ok_or_else(|| {
                    io::Error::new(ErrorKind::BrokenPipe, RECEIVE_TRANSPORT_CLOSED)
                })?;
                self.handle_frame(frame).await?;
            }
            // Biased selection gives ready retransmissions priority over new requests.
            () = ready(()), if retrying => self.retransmit_next().await?,
            request = self.requests.recv(), if can_accept_request => {
                if let Some(request) = request {
                    self.handle_request(request).await?;
                }
            }
        }
        Ok(())
    }

    /// Completes the initial reset handshake before taking application requests.
    async fn connect(&mut self) -> io::Result<()> {
        for attempt in 1..=RESET_ATTEMPTS {
            if self.requests.is_closed() && self.requests.is_empty() {
                return Ok(());
            }
            debug!("ASHv2 RST attempt {attempt}/{RESET_ATTEMPTS}");
            self.buffer.write_reset().await?;

            if let Ok(result) = timeout(T_RSTACK_MAX, self.wait_for_reset_ack()).await {
                result?;
                info!("ASHv2 link connected");
                return Ok(());
            }
        }
        Err(io::Error::new(
            ErrorKind::TimedOut,
            "ASHv2 reset acknowledgement timed out",
        ))
    }

    async fn wait_for_reset_ack(&mut self) -> io::Result<()> {
        while let Some(received) = self.frames.recv().await {
            if let Received::Frame(Frame::RstAck(ack)) = received
                && ack.validate().is_ok()
            {
                return if ack.is_ash_v2() {
                    Ok(())
                } else {
                    Err(io::Error::new(
                        ErrorKind::InvalidData,
                        format!("Unsupported ASH version {} in RSTACK", ack.version()),
                    ))
                };
            }
        }
        Err(io::Error::new(
            ErrorKind::BrokenPipe,
            RECEIVE_TRANSPORT_CLOSED,
        ))
    }

    async fn handle_frame(&mut self, received: Received) -> io::Result<()> {
        let Received::Frame(frame) = received else {
            return self.reject_frame().await;
        };
        let Ok(frame) = frame.validate() else {
            return self.reject_frame().await;
        };

        debug!("Received {frame}");
        match frame {
            Frame::Ack(ack) => {
                if !self.acknowledge(ack.ack_num()) {
                    self.reject_frame().await?;
                }
            }
            Frame::Nak(nak) => {
                if self.acknowledge(nak.ack_num()) {
                    self.start_retransmission();
                } else {
                    self.reject_frame().await?;
                }
            }
            Frame::Data(data) => {
                if self.acknowledge(data.ack_num()) {
                    self.handle_data(*data).await?;
                } else {
                    self.reject_frame().await?;
                }
            }
            Frame::Error(error) if !error.is_ash_v2() => self.reject_frame().await?,
            Frame::Rst(_) | Frame::RstAck(_) | Frame::Error(_) => {
                return Err(io::Error::new(
                    ErrorKind::ConnectionReset,
                    format!("Received {frame} on an established ASHv2 link"),
                ));
            }
        }
        Ok(())
    }

    /// Applies a cumulative ACK to the ordered window, including modulo-eight wraparound.
    fn acknowledge(&mut self, ack_num: u8) -> bool {
        let first = self
            .transmissions
            .first()
            .map_or(self.frame_number, Transmission::frame_num);
        let count = usize::from(ack_num.wrapping_sub(first) & SEQ_MASK);
        if count > self.transmissions.len() {
            debug!("ASHv2 ACK {ack_num} outside transmit window at {first}");
            return false;
        }
        for transmission in self.transmissions.drain(..count) {
            transmission.complete(Ok(()));
        }
        self.retries_remaining = self.retries_remaining.min(self.transmissions.len());
        if count > 0 {
            debug!(
                "ASHv2 ACK {ack_num} released {count} frames, {} outstanding",
                self.transmissions.len()
            );
        }
        true
    }

    /// Advances the receive sequence after buffering in-order DATA.
    async fn handle_data(&mut self, data: Data) -> io::Result<()> {
        if data.frame_num() != self.ack_number {
            return if data.is_retransmission() {
                self.send_readiness_ack().await
            } else {
                self.reject_frame().await
            };
        }

        let mut payload = data.into_payload();
        payload.mask();
        match self.response.try_send(payload) {
            Ok(()) => {
                self.ack_number = next_sequence_number(self.ack_number);
                self.rejecting = false;
            }
            Err(TrySendError::Full(_)) => return self.reject_frame().await,
            Err(TrySendError::Closed(_)) => {
                return Err(io::Error::new(
                    ErrorKind::BrokenPipe,
                    RESPONSE_CHANNEL_CLOSED,
                ));
            }
        }
        self.send_readiness_ack().await
    }

    async fn reject_frame(&mut self) -> io::Result<()> {
        if !self.rejecting {
            self.rejecting = true;
            let not_ready = self.response.capacity() == 0;
            self.buffer
                .write_frame(Nak::new(self.ack_number, not_ready))
                .await?;
            self.update_readiness_deadline(not_ready);
        }
        Ok(())
    }

    async fn send_readiness_ack(&mut self) -> io::Result<()> {
        let not_ready = self.response.capacity() == 0;
        self.buffer
            .write_frame(Ack::new(self.ack_number, not_ready))
            .await?;
        self.update_readiness_deadline(not_ready);
        Ok(())
    }

    fn update_readiness_deadline(&mut self, not_ready: bool) {
        self.not_ready_until = not_ready.then(|| Instant::now() + NOT_READY_REFRESH);
    }

    async fn handle_request(&mut self, request: Message) -> io::Result<()> {
        if request.response_tx.is_closed() {
            return Ok(());
        }

        let data = Data::new(self.frame_number, self.ack_number, *request.payload);
        let index = self.transmissions.len();
        self.transmissions
            .push(Transmission::new(data, request.response_tx))
            .map_err(|transmission| {
                transmission.complete(Err(io::Error::other(WINDOW_EXCEEDED)));
                io::Error::other(WINDOW_EXCEEDED)
            })?;
        self.buffer
            .write_data(&mut self.transmissions[index], self.ack_number)
            .await?;
        self.frame_number = next_sequence_number(self.frame_number);
        Ok(())
    }

    fn start_retransmission(&mut self) {
        if self.retries_remaining == 0 {
            self.retries_remaining = self.transmissions.len();
        }
    }

    async fn retransmit_next(&mut self) -> io::Result<()> {
        let index = self.transmissions.len() - self.retries_remaining;
        let transmission = &mut self.transmissions[index];
        debug!("Retransmitting ASHv2 frame {}", transmission.frame_num());
        self.buffer
            .write_data(transmission, self.ack_number)
            .await?;
        self.retries_remaining -= 1;
        Ok(())
    }
}
