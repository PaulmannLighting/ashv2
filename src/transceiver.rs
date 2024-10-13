mod buffers;
mod channels;
mod constants;

mod state;
mod transmission;

use crate::frame::Frame;
use crate::packet::{Ack, Data, Nak, Packet, RstAck, RST};
use crate::protocol::{AshChunks, Mask, Stuffing, CANCEL, FLAG, SUBSTITUTE, WAKE, X_OFF, X_ON};
use crate::status::Status;
use crate::utils::WrappingU3;
use crate::{HexSlice, Request};
use buffers::Buffers;
use channels::Channels;
use constants::{T_RSTACK_MAX, T_RX_ACK_MAX, T_RX_ACK_MIN};
use log::{debug, error, info, trace, warn};
use serialport::SerialPort;
use state::State;
use std::fmt::{LowerHex, UpperHex};
use std::io::{Error, ErrorKind, Read};
use std::slice::Chunks;
use std::sync::{
    atomic::{AtomicBool, Ordering::Relaxed},
    mpsc::{Receiver, SyncSender},
    Arc,
};
use std::task::Waker;
use std::time::{Duration, SystemTime};
use transmission::Transmission;

/// `ASHv2` transceiver.
///
/// The transceiver is responsible for handling the communication between the host and the NCP.
///
/// It is supposed to be run in a separate thread.
///
/// The [`AshFramed`](crate::AshFramed) struct implements a stream
/// to communicate with the NCP via the transceiver.
#[derive(Debug)]
pub struct Transceiver<T>
where
    T: SerialPort,
{
    serial_port: T,
    channels: Channels,
    buffers: Buffers,
    state: State,
}

impl<T> Transceiver<T>
where
    T: SerialPort,
{
    /// Create a new transceiver.
    ///
    /// # Parameters
    ///
    /// - `serial_port`: The serial port to communicate with the NCP.
    /// - `requests`: The channel to receive requests from the host.
    /// - `callback`: An optional channel to send callbacks from the NCP to.
    ///
    /// If no callback channel is provided, the transceiver will
    /// silently discard any callbacks actively sent from the NCP.
    #[must_use]
    pub fn new(
        serial_port: T,
        requests: Receiver<Request>,
        waker: Receiver<Waker>,
        callback: Option<SyncSender<Box<[u8]>>>,
    ) -> Self {
        Self {
            serial_port,
            channels: Channels::new(requests, waker, callback),
            buffers: Buffers::default(),
            state: State::new(),
        }
    }

    /// Run the transceiver.
    ///
    /// This should be called in a separate thread.
    #[allow(clippy::needless_pass_by_value)]
    pub fn run(mut self, running: Arc<AtomicBool>) {
        while running.load(Relaxed) {
            if let Err(error) = self.main() {
                self.handle_io_error(&error);
            }
        }
    }

    /// Main loop of the transceiver.
    ///
    /// This method checks whether the transceiver is connected and establishes a connection if not.
    /// Otherwise, it will communicate with the NCP via the `ASHv2` protocol.
    fn main(&mut self) -> std::io::Result<()> {
        match self.state.status {
            Status::Disconnected | Status::Failed => Ok(self.connect()?),
            Status::Connected => self.communicate(),
        }
    }

    /// Communicate with the NCP.
    ///
    /// If there is an incoming transaction, handle it.
    /// Otherwise, handle callbacks.
    fn communicate(&mut self) -> std::io::Result<()> {
        if let Some(bytes) = self.channels.receive()? {
            self.transaction(bytes.ash_chunks()?)
        } else {
            self.handle_callbacks()
        }
    }
}

/// Establish an `ASHv2` connection with the NCP.
impl<T> Transceiver<T>
where
    T: SerialPort,
{
    /// Establish an `ASHv2` connection with the NCP.
    fn connect(&mut self) -> std::io::Result<()> {
        debug!("Connecting to NCP...");
        let start = SystemTime::now();
        let mut attempts: usize = 0;

        'attempts: loop {
            attempts += 1;
            self.rst()?;

            debug!("Waiting for RSTACK...");
            let packet = loop {
                if let Some(packet) = self.receive()? {
                    break packet;
                } else if let Ok(elapsed) = start.elapsed() {
                    // Retry sending `RST` if no `RSTACK` was received in time.
                    if elapsed > T_RSTACK_MAX {
                        continue 'attempts;
                    }
                } else {
                    // If the system time jumps, retry sending `RST`.
                    error!("System time jumped.");
                    continue 'attempts;
                }
            };

            match packet {
                Packet::RstAck(rst_ack) => {
                    if !rst_ack.is_ash_v2() {
                        return Err(Error::new(
                            ErrorKind::Unsupported,
                            "ASHv2: Received RSTACK is not ASHv2.",
                        ));
                    }

                    self.state.status = Status::Connected;
                    info!(
                        "ASHv2 connection established after {attempts} attempt{}.",
                        if attempts > 1 { "s" } else { "" }
                    );

                    if let Ok(elapsed) = start.elapsed() {
                        debug!("Establishing connection took {elapsed:?}");
                    }

                    match rst_ack.code() {
                        Ok(code) => trace!("Received RST_ACK with code: {code}"),
                        Err(code) => warn!("Received RST_ACK with unknown code: {code}"),
                    }

                    return Ok(());
                }
                other => {
                    warn!("Expected RSTACK but got: {other}");
                    continue;
                }
            }
        }
    }
}

/// Transaction management for incoming commands.
///
/// This module handles incoming commands within transactions.
///
/// Incoming data is split into `ASH` chunks and sent to the NCP as long as the queue is not full.
/// Otherwise, the transactions waits for the NCP to acknowledge the sent data.
impl<T> Transceiver<T>
where
    T: SerialPort,
{
    /// Start a transaction of incoming data.
    fn transaction(&mut self, mut chunks: Chunks<'_, u8>) -> std::io::Result<()> {
        debug!("Starting transaction.");
        self.state.within_transaction = true;

        // Make sure that we do not receive any callbacks during the transaction.
        self.clear_callbacks()?;

        // Send chunks of data as long as there are chunks left to send.
        while self.send_chunks(&mut chunks)? {
            // Handle responses to sent chunks.
            while let Some(packet) = self.receive()? {
                self.handle_packet(packet)?;
            }

            // Retransmit timed out data.
            //
            // We do this here to avoid going into an infinite loop
            // if the NCP does not respond to out pushed chunks.
            while self.buffers.transmissions.is_full() {
                self.retransmit_timed_out_data()?;

                while let Some(packet) = self.receive()? {
                    self.handle_packet(packet)?;
                }
            }
        }

        // Handle any remaining responses.
        while let Some(packet) = self.receive()? {
            self.handle_packet(packet)?;
        }

        // Wait for retransmits to finish.
        while !self.buffers.transmissions.is_empty() {
            self.retransmit_timed_out_data()?;

            while let Some(packet) = self.receive()? {
                self.handle_packet(packet)?;
            }
        }

        debug!("Transaction completed.");
        self.state.within_transaction = false;

        // Send ACK without `nRDY` set, to re-enable callbacks.
        self.ack()?;

        // Close response channel.
        self.channels.close();
        Ok(())
    }

    /// Sends chunks as long as the retransmit queue is not full.
    ///
    /// Returns `true` if there are more chunks to send, otherwise `false`.
    fn send_chunks(&mut self, chunks: &mut Chunks<'_, u8>) -> std::io::Result<bool> {
        while !self.buffers.transmissions.is_full() {
            if let Some(chunk) = chunks.next() {
                self.send_chunk(chunk)?;
            } else {
                return Ok(false);
            }
        }

        Ok(true)
    }

    /// Sends a chunk of data.
    fn send_chunk(&mut self, chunk: &[u8]) -> std::io::Result<()> {
        let payload: heapless::Vec<u8, { Data::MAX_PAYLOAD_SIZE }> =
            chunk.try_into().map_err(|()| {
                Error::new(
                    ErrorKind::OutOfMemory,
                    "ASHv2: could not append chunk to frame buffer",
                )
            })?;
        let data = Data::new(
            self.state.next_frame_number(),
            payload,
            self.state.ack_number(),
        );
        self.transmit(data.into())
    }

    /// Clear any callbacks received before the transaction.
    fn clear_callbacks(&mut self) -> std::io::Result<()> {
        // Disable callbacks by sending an ACK with `nRDY` set.
        self.ack()?;

        while let Some(packet) = self.receive()? {
            self.handle_packet(packet)?;
        }

        Ok(())
    }
}

/// Handling of sent `DATA` frames.
///
/// This module handles acknowledgement and retransmission of sent `DATA` frames.
///
/// `ASH` retransmits `DATA` frames if they
///
///   * have been `NAK`ed by the NCP or
///   * not been acknowledged by the NCP in time.
impl<T> Transceiver<T>
where
    T: SerialPort,
{
    /// Remove `DATA` frames from the queue that have been acknowledged by the NCP.
    fn ack_sent_packets(&mut self, ack_num: WrappingU3) {
        while let Some(transmission) = self
            .buffers
            .transmissions
            .iter()
            .position(|transmission| transmission.frame_num() + 1 == ack_num)
            .map(|index| self.buffers.transmissions.remove(index))
        {
            if let Ok(duration) = transmission.elapsed() {
                trace!(
                    "ACKed packet {} after {duration:?}",
                    transmission.into_data()
                );
                self.update_t_rx_ack(Some(duration));
            } else {
                trace!("ACKed packet {}", transmission.into_data());
            }
        }
    }

    /// Retransmit `DATA` frames that have been `NAK`ed by the NCP.
    fn nak_sent_packets(&mut self, nak_num: WrappingU3) -> std::io::Result<()> {
        trace!("Handling NAK: {nak_num}");

        if let Some(transmission) = self
            .buffers
            .transmissions
            .iter()
            .position(|transmission| transmission.frame_num() == nak_num)
            .map(|index| self.buffers.transmissions.remove(index))
        {
            debug!("Retransmitting NAK'ed packet #{}", transmission.frame_num());
            self.transmit(transmission)?;
        }

        Ok(())
    }

    /// Retransmit `DATA` frames that have not been acknowledged by the NCP in time.
    fn retransmit_timed_out_data(&mut self) -> std::io::Result<()> {
        while let Some(transmission) = self
            .buffers
            .transmissions
            .iter()
            .position(|transmission| transmission.is_timed_out(self.state.t_rx_ack))
            .map(|index| self.buffers.transmissions.remove(index))
        {
            debug!(
                "Retransmitting timed-out packet #{}",
                transmission.frame_num()
            );
            self.update_t_rx_ack(None);
            self.transmit(transmission)?;
        }

        Ok(())
    }

    /// Update the `T_RX_ACK` timeout duration.
    fn update_t_rx_ack(&mut self, last_ack_duration: Option<Duration>) {
        self.state.t_rx_ack = last_ack_duration
            .map_or_else(
                || self.state.t_rx_ack * 2,
                |duration| self.state.t_rx_ack * 7 / 8 + duration / 2,
            )
            .clamp(T_RX_ACK_MIN, T_RX_ACK_MAX);
        trace!("Updated T_RX_ACK to {:?}", self.state.t_rx_ack);
    }
}

/// `ASHv2` frame I/O implementation.
///
/// This module contains the implementation of the `ASHv2` frame I/O operations.
impl<T> Transceiver<T>
where
    T: SerialPort,
{
    /// Receives a packet from the serial port.
    ///
    /// Returns `Ok(None)` if no packet was received within the timeout.
    ///
    /// # Errors
    ///
    /// Returns an [Error] if the serial port read operation failed.
    fn receive(&mut self) -> std::io::Result<Option<Packet>> {
        match self.read_packet() {
            Ok(packet) => Ok(Some(packet)),
            Err(error) => {
                if error.kind() == ErrorKind::TimedOut {
                    Ok(None)
                } else {
                    Err(error)
                }
            }
        }
    }

    /// Send an `ACK` frame with the given ACK number.
    ///
    /// # Errors
    ///
    /// Returns an [Error] if the serial port read operation failed.
    fn ack(&mut self) -> std::io::Result<()> {
        self.send_ack(&Ack::new(self.state.ack_number(), self.state.n_rdy()))
    }

    /// Send a `NAK` frame with the current ACK number.
    ///
    /// # Errors
    ///
    /// Returns an [Error] if the serial port read operation failed.
    fn nak(&mut self) -> std::io::Result<()> {
        self.send_nak(&Nak::new(self.state.ack_number(), self.state.n_rdy()))
    }

    /// Send a RST frame.
    ///
    /// # Errors
    ///
    /// Returns an [Error] if the serial port read operation failed.
    fn rst(&mut self) -> std::io::Result<()> {
        self.write_frame(&RST)
    }

    /// Send a `DATA` frame.
    ///
    /// # Errors
    ///
    /// Returns an [Error] if the serial port read operation failed.
    fn transmit(&mut self, mut transmission: Transmission) -> std::io::Result<()> {
        self.write_frame(transmission.data_for_transmit()?)?;
        self.buffers
            .transmissions
            .insert(0, transmission)
            .map_err(|_| {
                Error::new(
                    ErrorKind::OutOfMemory,
                    "ASHv2: failed to enqueue retransmit",
                )
            })
    }

    /// Send a raw `ACK` frame.
    fn send_ack(&mut self, ack: &Ack) -> std::io::Result<()> {
        if ack.not_ready() {
            self.state
                .last_n_rdy_transmission
                .replace(SystemTime::now());
        }

        debug!("Sending ACK: {ack}");
        self.write_frame(ack)
    }

    /// Send a raw `NAK` frame.
    fn send_nak(&mut self, nak: &Nak) -> std::io::Result<()> {
        if nak.not_ready() {
            self.state
                .last_n_rdy_transmission
                .replace(SystemTime::now());
        }

        debug!("Sending NAK: {nak}");
        self.write_frame(nak)
    }

    /// Read an ASH [`Packet`].
    ///
    /// # Errors
    ///
    /// Returns an [`Error`] if any I/O, protocol or parsing error occurs.
    fn read_packet(&mut self) -> std::io::Result<Packet> {
        self.buffer_frame()?;
        Packet::try_from(self.buffers.frame.as_slice())
    }

    /// Reads an ASH frame into the transceiver's frame buffer.
    ///
    /// # Errors
    /// Returns an [`Error`] if any I/O or protocol error occurs.
    fn buffer_frame(&mut self) -> std::io::Result<()> {
        let buffer = &mut self.buffers.frame;
        buffer.clear();
        let serial_port = &mut self.serial_port;
        let mut error = false;

        for byte in serial_port.bytes() {
            match byte? {
                CANCEL => {
                    trace!("Resetting buffer due to cancel byte.");
                    buffer.clear();
                    error = false;
                }
                FLAG => {
                    trace!("Received flag byte.");

                    if !error && !buffer.is_empty() {
                        debug!("Received frame.");
                        trace!("Buffer: {:#04X}", HexSlice::new(buffer));
                        buffer.unstuff();
                        trace!("Unstuffed buffer: {:#04X}", HexSlice::new(buffer));
                        return Ok(());
                    }

                    trace!("Resetting buffer due to error or empty buffer.");
                    trace!("Error condition was: {error}");
                    trace!("Buffer: {:#04X}", HexSlice::new(buffer));
                    buffer.clear();
                    error = false;
                }
                SUBSTITUTE => {
                    trace!("Received SUBSTITUTE byte. Setting error condition.");
                    error = true;
                }
                X_ON => {
                    warn!("NCP requested to resume transmission. Ignoring.");
                }
                X_OFF => {
                    warn!("NCP requested to stop transmission. Ignoring.");
                }
                WAKE => {
                    debug!("NCP tried to wake us up.");
                }
                byte => {
                    if buffer.push(byte).is_err() {
                        return Err(Error::new(
                            ErrorKind::OutOfMemory,
                            "ASHv2: frame buffer overflow",
                        ));
                    }
                }
            }
        }

        Err(Error::new(
            ErrorKind::UnexpectedEof,
            "ASHv2: Byte stream terminated unexpectedly",
        ))
    }

    /// Writes an ASH [`Frame`].
    ///
    /// # Errors
    ///
    /// Returns an [Error] if the serial port write operation failed.
    fn write_frame<F>(&mut self, frame: &F) -> std::io::Result<()>
    where
        F: Frame + LowerHex + UpperHex,
    {
        let buffer = &mut self.buffers.frame;
        debug!("Writing frame: {frame}");
        trace!("Frame: {frame:#04X}");
        trace!("Frame: {frame:#04x}");
        buffer.clear();
        frame.buffer(buffer).map_err(|()| {
            Error::new(
                ErrorKind::OutOfMemory,
                "ASHv2: Could not append frame bytes to buffer.",
            )
        })?;
        trace!("Frame bytes: {:#04X}", HexSlice::new(buffer));
        buffer.stuff()?;
        trace!("Stuffed bytes: {:#04X}", HexSlice::new(buffer));
        buffer.push(FLAG).map_err(|_| {
            Error::new(ErrorKind::OutOfMemory, "ASHv2: Could not append flag byte.")
        })?;
        trace!("Writing bytes: {:#04X}", HexSlice::new(buffer));
        self.serial_port.write_all(buffer)?;
        self.serial_port.flush()
    }
}

/// Packet handling implementation for the transceiver.
///
/// This module contains methods to handle incoming packets sent by the NCP.
impl<T> Transceiver<T>
where
    T: SerialPort,
{
    /// Handle an incoming packet.
    ///
    /// # Errors
    ///
    /// Returns a [Error] if the packet handling failed.
    fn handle_packet(&mut self, packet: Packet) -> std::io::Result<()> {
        debug!("Handling: {packet}");
        trace!("{packet:#04X}");
        trace!("{packet:#04x}");

        if self.state.status == Status::Connected {
            match packet {
                Packet::Ack(ref ack) => self.handle_ack(ack),
                Packet::Data(data) => self.handle_data(data)?,
                Packet::Error(ref error) => return Err(Self::handle_error(error)),
                Packet::Nak(ref nak) => self.handle_nak(nak)?,
                Packet::RstAck(ref rst_ack) => return Err(Self::handle_rst_ack(rst_ack)),
                Packet::Rst(_) => warn!("Received unexpected RST from NCP."),
            }
        } else {
            warn!("Not connected. Dropping frame: {packet}");
        }

        Ok(())
    }

    /// Handle an incoming `ACK` packet.
    fn handle_ack(&mut self, ack: &Ack) {
        if !ack.is_crc_valid() {
            warn!("Received ACK with invalid CRC.");
        }

        self.ack_sent_packets(ack.ack_num());
    }

    /// Handle an incoming `DATA` packet.
    fn handle_data(&mut self, data: Data) -> std::io::Result<()> {
        if !data.is_crc_valid() {
            warn!("Received data frame with invalid CRC.");
            self.enter_reject()?;
        } else if data.frame_num() == self.state.ack_number() {
            self.leave_reject();
            self.state.last_received_frame_num.replace(data.frame_num());
            self.ack()?;
            self.ack_sent_packets(data.ack_num());
            self.handle_payload(data.into_payload());
        } else if data.is_retransmission() {
            warn!("Received retransmission of frame: {data}");
            self.ack_sent_packets(data.ack_num());
            self.handle_payload(data.into_payload());
        } else {
            debug!("Received out-of-sequence data frame: {data}");
            self.enter_reject()?;
        }

        Ok(())
    }

    /// Extends the response buffer with the given data.
    fn handle_payload(&mut self, mut payload: heapless::Vec<u8, { Data::MAX_PAYLOAD_SIZE }>) {
        payload.mask();
        self.channels.respond(payload.as_slice().into());
    }

    /// Handle an incoming `ERROR` packet.
    fn handle_error(error: &crate::packet::Error) -> Error {
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

        Error::new(
            ErrorKind::ConnectionReset,
            "ASHv2: NCP entered ERROR state.",
        )
    }

    /// Handle an incoming `NAK` packet.
    fn handle_nak(&mut self, nak: &Nak) -> std::io::Result<()> {
        if !nak.is_crc_valid() {
            warn!("Received ACK with invalid CRC.");
        }

        self.nak_sent_packets(nak.ack_num())
    }

    /// Handle an incoming `RSTACK` packet.
    fn handle_rst_ack(rst_ack: &RstAck) -> Error {
        error!("Received unexpected RSTACK: {rst_ack}");

        if !rst_ack.is_ash_v2() {
            error!("{rst_ack} is not ASHv2: {:#04X}", rst_ack.version());
        }

        rst_ack.code().map_or_else(
            |code| {
                error!("NCP sent RSTACK with unknown code: {code}");
            },
            |code| {
                warn!("NCP sent RSTACK condition: {code}");
            },
        );

        Error::new(
            ErrorKind::ConnectionReset,
            "ASHv2: NCP received unexpected RSTACK.",
        )
    }
}

/// Handle callbacks actively sent by the NCP outside of transactions.
impl<T> Transceiver<T>
where
    T: SerialPort,
{
    /// Handle callbacks actively sent by the NCP outside of transactions.
    fn handle_callbacks(&mut self) -> std::io::Result<()> {
        while let Some(callback) = self.receive()? {
            self.handle_packet(callback)?;
        }

        Ok(())
    }
}

/// Reject state management.
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
///
impl<T> Transceiver<T>
where
    T: SerialPort,
{
    /// Enter the rejection state.
    fn enter_reject(&mut self) -> std::io::Result<()> {
        if self.state.reject {
            Ok(())
        } else {
            trace!("Entering rejection state.");
            self.state.reject = true;
            self.nak()
        }
    }

    /// Leave the rejection state.
    fn leave_reject(&mut self) {
        if self.state.reject {
            trace!("Leaving rejection state.");
            self.state.reject = false;
        }
    }
}

/// Reset and error handling implementation.
impl<T> Transceiver<T>
where
    T: SerialPort,
{
    /// Reset buffers and state.
    fn reset(&mut self) {
        self.channels.reset();
        self.buffers.clear();
        self.state.reset(Status::Failed);
    }

    /// Handle I/O errors.
    fn handle_io_error(&mut self, error: &Error) {
        error!("I/O error: {error}");

        if self.state.within_transaction {
            error!("Aborting current transaction due to error.");
            self.channels.close();
        }

        self.reset();
    }
}
