use std::io;
use std::io::ErrorKind;
use std::time::{Duration, Instant};

use log::{debug, error, info, trace, warn};
use serialport::SerialPort;
use tokio::sync::mpsc::{Receiver, Sender};
use tokio::sync::oneshot;

use self::buffer::Buffer;
use self::transmission::Transmission;
use crate::actor::message::Message;
use crate::frame::{Ack, Data, Error, Nak, RST, Rst, RstAck};
use crate::status::Status;
use crate::types::Payload;
use crate::utils::WrappingU3;

mod buffer;
mod transmission;

/// Maximum time to wait for RST ACK frame after sending RST frame.
const T_RSTACK_MAX: Duration = Duration::from_millis(3200);

/// The amount of maximum unacknowledged frames that the NCP (or Host) can hold.
/// Also amounts to the so-called *sliding window size*.
const TX_K: usize = 5;

const T_RX_ACK_INIT: Duration = Duration::from_millis(1600);
const T_RX_ACK_MIN: Duration = Duration::from_millis(400);
const T_RX_ACK_MAX: Duration = Duration::from_millis(3200);

/// `ASHv2` transmitter.
#[derive(Debug)]
pub struct Transmitter<T> {
    buffer: Buffer<T>,
    messages: Receiver<Message>,
    requeue: Sender<Message>,
    status: Status,
    last_rst_sent: Option<Instant>,
    transmissions: heapless::Vec<Transmission, TX_K>,
    frame_number: WrappingU3,
    ack_number: WrappingU3,
    t_rx_ack: Duration,
}

impl<T> Transmitter<T> {
    /// Creates a new `ASHv2` transmitter.
    #[must_use]
    pub const fn new(
        serial_port: T,
        messages: Receiver<Message>,
        requeue: Sender<Message>,
    ) -> Self {
        Self {
            buffer: Buffer::new(serial_port),
            messages,
            requeue,
            status: Status::Disconnected,
            last_rst_sent: None,
            transmissions: heapless::Vec::new(),
            frame_number: WrappingU3::ZERO,
            ack_number: WrappingU3::ZERO,
            t_rx_ack: T_RX_ACK_INIT,
        }
    }
}

impl<T> Transmitter<T>
where
    T: SerialPort + 'static,
{
    pub async fn run(mut self) {
        while let Some(message) = self.messages.recv().await {
            if let Err(error) = self.handle_message(message).await {
                error!("Resetting connection due to I/O error: {error}");
                self.status = Status::Failed;
            }
        }

        info!("Message channel closed, transmitter exiting.");
    }

    async fn handle_message(&mut self, message: Message) -> io::Result<()> {
        trace!("Received message to transmit: {message:?}");

        if self.status != Status::Connected {
            if let Message::RstAck(ack) = message {
                return self.handle_rst_ack(&ack);
            }

            warn!("Transmitter not connected (status: {:?}).", self.status);
            trace!("Requeuing message: {message:?}");
            self.requeue.send(message).await.unwrap_or_else(|error| {
                error!("Failed to requeue message: {error}");
            });
            return Ok(());
        }

        match message {
            Message::Payload { payload, response } => self.handle_payload(&payload, response).await,
            Message::Ack(ack_num) => self.send_ack(ack_num),
            Message::Nak(ack_num) => self.send_nak(ack_num),
            Message::Rst(rst) => self.handle_rst(&rst),
            Message::RstAck(rst_ack) => self.handle_rst_ack(&rst_ack),
            Message::Error(error) => self.handle_error(&error),
            Message::AckSentFrame(frame_num) => {
                self.ack_sent_frames(frame_num);
                Ok(())
            }
            Message::NakSentFrame(frame_num) => self.nak_sent_frames(frame_num),
        }
    }

    async fn handle_payload(
        &mut self,
        payload: &[u8],
        response: oneshot::Sender<io::Result<()>>,
    ) -> io::Result<()> {
        let chunks = payload.chunks(Data::MAX_PAYLOAD_SIZE);

        // Payload too large to fit into transmission queue.
        if chunks.len() > self.transmissions.capacity() {
            response
                .send(Err(io::Error::new(
                    ErrorKind::OutOfMemory,
                    "Insufficient space in transmission queue",
                )))
                .unwrap_or_else(|_| {
                    error!("Failed to send payload response.");
                });

            return Ok(());
        }

        // Not enough space in transmission queue, requeue the payload.
        if chunks.len()
            > self
                .transmissions
                .capacity()
                .saturating_sub(self.transmissions.len())
        {
            warn!("Insufficient space in transmission queue for payload, requeuing...");
            self.requeue
                .send(Message::Payload {
                    payload: Box::from(payload),
                    response,
                })
                .await
                .unwrap_or_else(|error| {
                    error!("Failed to requeue payload message: {error}");
                });
        }

        // Transmit the chunks.
        for (index, chunk) in chunks
            .map(|chunk| Payload::try_from(chunk).expect("Chunk size fits into Payload"))
            .enumerate()
        {
            let data = Data::new(self.next_frame_number(), chunk, self.ack_number + index);
            self.transmit(data.into())?;
        }

        Ok(())
    }

    fn send_ack(&mut self, ack_num: WrappingU3) -> io::Result<()> {
        self.ack_number = ack_num;
        self.buffer.write_frame(Ack::new(ack_num, false))
    }

    fn send_nak(&mut self, ack_num: WrappingU3) -> io::Result<()> {
        self.buffer.write_frame(Nak::new(ack_num, false))
    }

    /// Handle RST frame received from the NCP.
    fn handle_rst(&mut self, rst: &Rst) -> io::Result<()> {
        error!("Received RST frame: {rst}, resetting connection.");
        self.status = Status::Disconnected;
        self.reset()
    }

    /// Handle RST ACK frame received from the NCP.
    fn handle_rst_ack(&mut self, rst_ack: &RstAck) -> io::Result<()> {
        trace!("Received RST ACK frame: {rst_ack}, connection reset acknowledged.");

        if let Some(timestamp) = self.last_rst_sent.take() {
            if timestamp.elapsed() < T_RSTACK_MAX {
                debug!("Connection established successfully.");
                self.status = Status::Connected;
                Ok(())
            } else {
                warn!("RST ACK received after timeout. Resetting connection again.");
                self.reset()
            }
        } else {
            warn!("Received unexpected RST ACK frame: {rst_ack}.");
            Ok(())
        }
    }

    /// Handle errors received from the NCP.
    fn handle_error(&mut self, error: &Error) -> io::Result<()> {
        warn!("Transmitter encountered error: {error}, resetting connection.");
        self.status = Status::Failed;
        self.reset()
    }

    /// Remove `DATA` frames from the queue that have been acknowledged by the NCP.
    fn ack_sent_frames(&mut self, ack_num: WrappingU3) {
        // Remove timed-out transmissions.
        self.transmissions
            .retain(|transmission| !transmission.is_timed_out(T_RX_ACK_MAX));

        // Remove acknowledged transmissions.
        while let Some(transmission) = self
            .transmissions
            .iter()
            .position(|transmission| transmission.frame_num() + 1u8 == ack_num)
            .map(|index| self.transmissions.remove(index))
        {
            let duration = transmission.elapsed();
            trace!("ACKed frame {transmission} after {duration:?}");
            self.update_t_rx_ack(Some(duration));
        }
    }

    /// Retransmit `DATA` frames that have been `NAK`ed by the NCP.
    fn nak_sent_frames(&mut self, nak_num: WrappingU3) -> io::Result<()> {
        // Remove timed-out transmissions.
        self.transmissions
            .retain(|transmission| !transmission.is_timed_out(T_RX_ACK_MAX));

        // Retransmit NAK'ed transmission.
        if let Some(transmission) = self
            .transmissions
            .iter()
            .position(|transmission| transmission.frame_num() == nak_num)
            .map(|index| self.transmissions.remove(index))
        {
            debug!("Retransmitting NAK'ed frame #{}", transmission.frame_num());
            self.transmit(transmission)?;
        }

        Ok(())
    }

    /// Send a `DATA` frame.
    fn transmit(&mut self, mut transmission: Transmission) -> io::Result<()> {
        let data = transmission.data_for_transmit()?;
        trace!("Transmitting frame {data:#04X}");
        self.buffer.write_frame(data)?;
        self.transmissions
            .insert(0, transmission)
            .map_err(|_| io::Error::new(ErrorKind::OutOfMemory, "Failed to enqueue retransmit"))
    }

    /// Send RST frame to reset the connection.
    fn reset(&mut self) -> io::Result<()> {
        if let Some(timestamp) = self.last_rst_sent.take()
            && timestamp.elapsed() < T_RSTACK_MAX
        {
            debug!("Last RST sent {timestamp:?} ago, waiting before sending another...");
            self.last_rst_sent.replace(timestamp);
            return Ok(());
        }

        self.last_rst_sent.replace(Instant::now());
        self.buffer.write_frame(RST)
    }

    /// Returns the next frame number.
    pub fn next_frame_number(&mut self) -> WrappingU3 {
        let frame_number = self.frame_number;
        self.frame_number += 1;
        frame_number
    }

    /// Update the `T_RX_ACK` timeout duration.
    fn update_t_rx_ack(&mut self, last_ack_duration: Option<Duration>) {
        self.t_rx_ack = last_ack_duration
            .map_or_else(
                || self.t_rx_ack * 2,
                |duration| self.t_rx_ack * 7 / 8 + duration / 2,
            )
            .clamp(T_RX_ACK_MIN, T_RX_ACK_MAX);
        trace!("Updated T_RX_ACK to {:?}", self.t_rx_ack);
    }
}
