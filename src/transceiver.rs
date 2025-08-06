//! Transceiver for the `ASHv2` protocol.

use std::io::{self, Error, ErrorKind, Read, Write};
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering::Relaxed;
use std::thread::{JoinHandle, sleep, spawn};
use std::time::{Duration, Instant};

use channels::Channels;
use constants::{T_RSTACK_MAX, TX_K};
use log::{debug, error, info, trace, warn};
use state::State;
use tokio::sync::mpsc::{Receiver, Sender, channel};
use transmission::Transmission;

use crate::frame::{Ack, Data, Frame, Nak, RST, RstAck};
use crate::frame_buffer::FrameBuffer;
use crate::protocol::Mask;
use crate::status::Status;
use crate::types::Payload;
use crate::utils::WrappingU3;
use crate::validate::Validate;

const CONNECT_GRACE_TIME_FACTOR: Duration = Duration::from_millis(100);
const MAX_CONNECTION_GRACE_TIME: Duration = Duration::from_secs(5);

mod channels;
mod constants;
mod state;
mod transmission;

/// `ASHv2` transceiver.
///
/// The transceiver is responsible for handling the communication between the host and the NCP.
///
/// It is supposed to be run in a separate thread.
#[derive(Debug)]
pub struct Transceiver<T> {
    frame_buffer: FrameBuffer<T>,
    channels: Channels,
    state: State,
    transmissions: heapless::Vec<Transmission, TX_K>,
}

impl<T> Transceiver<T> {
    /// Create a new transceiver.
    #[must_use]
    pub const fn new(
        serial_port: T,
        requests: Receiver<Payload>,
        response: Sender<io::Result<Payload>>,
    ) -> Self {
        Self {
            frame_buffer: FrameBuffer::new(serial_port),
            channels: Channels::new(requests, response),
            state: State::new(),
            transmissions: heapless::Vec::new(),
        }
    }

    /// Reset buffers and state.
    fn reset(&mut self) {
        self.state.reset(Status::Failed);
        self.transmissions.clear();
    }

    /// Handle I/O errors.
    fn handle_io_error(&mut self, error: Error) {
        debug!("Handling I/O error: {error}");
        self.channels.respond(Err(error));
        self.reset();
    }

    /// Leave the rejection state.
    fn leave_reject(&mut self) {
        if self.state.reject() {
            trace!("Leaving rejection state.");
            self.state.set_reject(false);
        }
    }

    /// Send the response frame's payload through the response channel.
    fn handle_payload(&self, mut payload: Payload) {
        payload.mask();
        self.channels.respond(Ok(payload));
    }

    /// Handle an incoming `RSTACK` frame.
    fn handle_rst_ack(rst_ack: &RstAck) -> Error {
        debug!("Received RSTACK: {rst_ack}");

        if !rst_ack.is_ash_v2() {
            error!("{rst_ack} is not ASHv2: {:#04X}", rst_ack.version());
        }

        rst_ack.code().map_or_else(
            |code| {
                warn!("NCP sent RSTACK with unknown code: {code}");
            },
            |code| {
                trace!("NCP sent RSTACK condition: {code}");
            },
        );

        Error::new(ErrorKind::ConnectionReset, "NCP sent RSTACK.")
    }

    /// Handle an incoming `ERROR` frame.
    fn handle_error(error: &crate::frame::Error) -> Error {
        if !error.is_ash_v2() {
            error!("{error} is not ASHv2: {:#04X}", error.version());
        }

        error.code().map_or_else(
            |code| {
                error!("NCP sent ERROR with invalid code: {code}");
            },
            |code| {
                warn!("NCP sent ERROR condition: {code}");
            },
        );

        Error::new(ErrorKind::ConnectionReset, "NCP entered ERROR state.")
    }
}

impl<T> Transceiver<T>
where
    T: Write,
{
    /// Remove `DATA` frames from the queue that have been acknowledged by the NCP.
    fn ack_sent_frames(&mut self, ack_num: WrappingU3) {
        while let Some(transmission) = self
            .transmissions
            .iter()
            .position(|transmission| transmission.frame_num() + 1 == ack_num)
            .map(|index| self.transmissions.remove(index))
        {
            let duration = transmission.elapsed();
            trace!("ACKed frame {transmission} after {duration:?}");
            self.state.update_t_rx_ack(Some(duration));
        }
    }

    /// Retransmit `DATA` frames that have been `NAK`ed by the NCP.
    fn nak_sent_frames(&mut self, nak_num: WrappingU3) -> io::Result<()> {
        trace!("Handling NAK: {nak_num}");

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

    /// Retransmit `DATA` frames that have not been acknowledged by the NCP in time.
    fn retransmit_timed_out_data(&mut self) -> io::Result<()> {
        while let Some(transmission) = self
            .transmissions
            .iter()
            .position(|transmission| transmission.is_timed_out(self.state.t_rx_ack()))
            .map(|index| self.transmissions.remove(index))
        {
            debug!(
                "Retransmitting timed-out frame #{}",
                transmission.frame_num()
            );
            self.state.update_t_rx_ack(None);
            self.transmit(transmission)?;
        }

        Ok(())
    }

    /// Send an `ACK` frame with the given ACK number.
    fn ack(&mut self) -> io::Result<()> {
        self.send_ack(Ack::new(self.state.ack_number(), false))
    }

    /// Send a `NAK` frame with the current ACK number.
    fn nak(&mut self) -> io::Result<()> {
        self.send_nak(Nak::new(self.state.ack_number(), false))
    }

    /// Send a RST frame.
    fn rst(&mut self) -> io::Result<()> {
        self.frame_buffer.write_frame(RST)
    }

    /// Send a `DATA` frame.
    fn transmit(&mut self, mut transmission: Transmission) -> io::Result<()> {
        let data = transmission.data_for_transmit()?;
        trace!("Unmasked {:#04X}", data.unmasked());
        self.frame_buffer.write_frame(data)?;
        self.transmissions
            .insert(0, transmission)
            .map_err(|_| Error::new(ErrorKind::OutOfMemory, "Failed to enqueue retransmit"))
    }

    /// Send a chunk of data.
    fn send_chunk(&mut self, chunk: Payload, offset: WrappingU3) -> io::Result<()> {
        let data = Data::new(
            self.state.next_frame_number(),
            chunk,
            self.state.ack_number() + offset,
        );
        self.transmit(data.into())
    }

    /// Send chunks as long as the retransmission queue is not full.
    ///
    /// Return `true` if there are more chunks to send, otherwise `false`.
    fn send_chunks(&mut self) -> io::Result<bool> {
        // With a sliding windows size > 1 the NCP may enter an "ERROR: Assert" state when sending
        // fragmented messages if each DATA frame's ACK number is not increased.
        let mut offset = WrappingU3::default();

        while !self.transmissions.is_full() {
            if let Some(chunk) = self.channels.receive()? {
                self.send_chunk(chunk, offset)?;
                offset += 1;
            } else {
                return Ok(false);
            }
        }

        Ok(true)
    }

    /// Send a raw `ACK` frame.
    fn send_ack(&mut self, ack: Ack) -> io::Result<()> {
        debug!("Sending ACK: {ack}");
        self.frame_buffer.write_frame(ack)
    }

    /// Send a raw `NAK` frame.
    fn send_nak(&mut self, nak: Nak) -> io::Result<()> {
        debug!("Sending NAK: {nak}");
        self.frame_buffer.write_frame(nak)
    }

    /// Enter the reject state.
    ///
    /// `ASH` sets the Reject Condition after receiving a `DATA` frame
    /// with any of the following attributes:
    ///
    ///   * Has an incorrect CRC.
    ///   * Has an invalid control byte.
    ///   * Is an invalid length for the frame type.
    ///   * Contains a low-level communication error (e.g., framing, overrun, or overflow).
    ///   * Has an invalid ackNum.
    ///   * Is out of sequence.
    ///   * Was valid, but had to be discarded due to lack of memory to store it.
    fn enter_reject(&mut self) -> io::Result<()> {
        if self.state.reject() {
            Ok(())
        } else {
            trace!("Entering rejection state.");
            self.state.set_reject(true);
            self.nak()
        }
    }

    /// Handle an incoming `DATA` frame.
    fn handle_data(&mut self, data: Data) -> io::Result<()> {
        trace!("Unmasked data: {:#04X}", data.unmasked());

        if !data.is_crc_valid() {
            warn!("Received data frame with invalid CRC.");
            self.enter_reject()?;
        } else if data.frame_num() == self.state.ack_number() {
            self.leave_reject();
            self.state.set_last_received_frame_num(data.frame_num());
            self.ack()?;
            self.ack_sent_frames(data.ack_num());
            self.handle_payload(data.into_payload());
        } else if data.is_retransmission() {
            info!("Received retransmission of frame: {data}");
            self.ack()?;
            self.ack_sent_frames(data.ack_num());
            self.handle_payload(data.into_payload());
        } else {
            warn!("Received out-of-sequence data frame: {data}");
            self.enter_reject()?;
        }

        Ok(())
    }

    /// Handle an incoming `NAK` frame.
    fn handle_nak(&mut self, nak: &Nak) -> io::Result<()> {
        if !nak.is_crc_valid() {
            warn!("Received ACK with invalid CRC.");
        }

        self.nak_sent_frames(nak.ack_num())
    }

    /// Handle an incoming `ACK` frame.
    fn handle_ack(&mut self, ack: &Ack) {
        if !ack.is_crc_valid() {
            warn!("Received ACK with invalid CRC.");
        }

        self.ack_sent_frames(ack.ack_num());
    }
}

impl<T> Transceiver<T>
where
    T: Read + Write,
{
    /// Spawn a new transceiver.
    ///
    /// # Returns
    ///
    /// Returns a tuple of the request sender, response receiver, and the transceiver thread handle.
    pub fn spawn(
        serial_port: T,
        running: Arc<AtomicBool>,
        channel_size: usize,
    ) -> (
        Sender<Payload>,
        Receiver<io::Result<Payload>>,
        JoinHandle<T>,
    )
    where
        T: Send + 'static,
    {
        let (request_tx, request_rx) = channel(channel_size);
        let (response_tx, response_rx) = channel(channel_size);
        let transceiver = Self::new(serial_port, request_rx, response_tx);
        (request_tx, response_rx, spawn(|| transceiver.run(running)))
    }

    /// Run the transceiver.
    ///
    /// This should be called in a separate thread.
    ///
    /// # Returns
    ///
    /// Returns the inner serial port after the transceiver has stopped running.
    #[allow(clippy::needless_pass_by_value)]
    pub fn run(mut self, running: Arc<AtomicBool>) -> T {
        while running.load(Relaxed) {
            if let Err(error) = self.main() {
                self.handle_io_error(error);
            }
        }

        self.frame_buffer.into_inner()
    }

    /// Main loop of the transceiver.
    ///
    /// This method checks whether the transceiver is connected and establishes a connection if not.
    /// Otherwise, it will communicate with the NCP via the `ASHv2` protocol.
    fn main(&mut self) -> io::Result<()> {
        match self.state.status() {
            Status::Disconnected | Status::Failed => Ok(self.connect()?),
            Status::Connected => self.communicate(),
        }
    }

    /// Communicate with the NCP.
    ///
    /// If there is an incoming transaction, handle it.
    /// Otherwise, handle callbacks.
    fn communicate(&mut self) -> io::Result<()> {
        self.send_data()?;
        self.handle_callbacks()
    }

    /// Establish an `ASHv2` connection with the NCP.
    fn connect(&mut self) -> io::Result<()> {
        debug!("Connecting to NCP...");
        let start = Instant::now();

        'attempts: for attempt in 1usize.. {
            wait_before_attempt(attempt);
            self.rst()?;

            debug!("Waiting for RSTACK...");
            let frame = loop {
                if let Some(frame) = self.receive()? {
                    break frame;
                } else if start.elapsed() > T_RSTACK_MAX {
                    continue 'attempts;
                }
            };

            match frame {
                Frame::RstAck(rst_ack) => {
                    if !rst_ack.is_ash_v2() {
                        return Err(Error::new(
                            ErrorKind::Unsupported,
                            "Received RSTACK is not ASHv2.",
                        ));
                    }

                    self.state.set_status(Status::Connected);
                    info!(
                        "ASHv2 connection established after {attempt} attempt{}.",
                        if attempt > 1 { "s" } else { "" }
                    );

                    debug!("Establishing connection took {:?}", start.elapsed());

                    match rst_ack.code() {
                        Ok(code) => trace!("Received RST_ACK with code: {code}"),
                        Err(code) => warn!("Received RST_ACK with unknown code: {code}"),
                    }

                    return Ok(());
                }
                other => {
                    warn!("Expected RSTACK but got: {other}");
                }
            }
        }

        Err(Error::new(
            ErrorKind::ConnectionRefused,
            "Connection refused",
        ))
    }

    /// Receive a frame from the serial port.
    ///
    /// Return `Ok(None)` if no frame was received within the timeout.
    fn receive(&mut self) -> io::Result<Option<Frame>> {
        let frame = match self.frame_buffer.read_frame() {
            Ok(frame) => frame,
            Err(error) => {
                if error.kind() == ErrorKind::TimedOut {
                    return Ok(None);
                }

                return Err(error);
            }
        };

        if frame.is_crc_valid() {
            return Ok(Some(frame));
        }

        trace!("Received frame with invalid CRC checksum: {frame:#04X}");

        if let Frame::Data(_) = frame {
            self.enter_reject()?;
        }

        Ok(None)
    }

    /// Start a transaction of incoming data.
    fn send_data(&mut self) -> io::Result<()> {
        // Send chunks of data as long as there are chunks left to send.
        while self.send_chunks()? {
            // Wait for space in the queue to become available before transmitting more data.
            while self.transmissions.is_full() {
                // Handle potential incoming ACKs and DATA frames.
                while let Some(frame) = self.receive()? {
                    self.handle_frame(frame)?;
                }

                // Retransmit timed out data.
                //
                // We do this here to avoid going into an infinite loop
                // if the NCP does not respond to our pushed chunks.
                self.retransmit_timed_out_data()?;
            }
        }

        // Wait for retransmits to finish.
        while !self.transmissions.is_empty() {
            while let Some(frame) = self.receive()? {
                self.handle_frame(frame)?;
            }

            self.retransmit_timed_out_data()?;
        }

        Ok(())
    }

    /// Handle an incoming frame.
    fn handle_frame(&mut self, frame: Frame) -> io::Result<()> {
        debug!("Handling: {frame}");
        trace!("{frame:#04X}");

        if self.state.status() == Status::Connected {
            match frame {
                Frame::Ack(ref ack) => self.handle_ack(ack),
                Frame::Data(data) => self.handle_data(*data)?,
                Frame::Error(ref error) => return Err(Self::handle_error(error)),
                Frame::Nak(ref nak) => self.handle_nak(nak)?,
                Frame::RstAck(ref rst_ack) => return Err(Self::handle_rst_ack(rst_ack)),
                Frame::Rst(_) => warn!("Received unexpected RST from NCP."),
            }
        } else {
            warn!("Not connected. Dropping frame: {frame}");
        }

        Ok(())
    }

    /// Handle callbacks actively sent by the NCP outside of transactions.
    fn handle_callbacks(&mut self) -> io::Result<()> {
        while let Some(callback) = self.receive()? {
            self.handle_frame(callback)?;
        }

        Ok(())
    }
}

/// Wait before performing the next connection attempt.
fn wait_before_attempt(attempt: usize) {
    if attempt > 1 {
        let grace_time = u32::try_from(attempt)
            .map(|attempts| (CONNECT_GRACE_TIME_FACTOR * attempts).max(MAX_CONNECTION_GRACE_TIME))
            .unwrap_or(MAX_CONNECTION_GRACE_TIME);
        debug!("Sleeping for {grace_time:?} before next connection attempt.");
        sleep(grace_time);
    }
}
