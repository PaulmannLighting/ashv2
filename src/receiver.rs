use std::fmt::{LowerHex, UpperHex};
use std::io::{Error, ErrorKind, Read};
use std::slice::Chunks;
use std::sync::{
    atomic::{AtomicBool, Ordering::Relaxed},
    mpsc::{Receiver, SyncSender},
    Arc, RwLock, RwLockWriteGuard,
};
use std::task::Waker;
use std::time::SystemTime;

use crate::constants::T_RSTACK_MAX;
use crate::frame::Frame;
use crate::packet::{Ack, Data, Nak, Packet, RstAck, RST};
use crate::protocol::{AshChunks, Mask, Stuffing, CANCEL, FLAG, SUBSTITUTE, WAKE, X_OFF, X_ON};
use crate::request::Request;
use crate::response::Response;
use crate::status::Status;
use crate::types::FrameBuffer;
use crate::utils::WrappingU3;
use crate::write_frame::WriteFrame;
use crate::{HexSlice, Payload};
use log::{debug, error, info, trace, warn};
use serialport::SerialPort;
use tokio::sync::mpsc::Sender;

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
    response: Sender<Response>,
    status: Arc<RwLock<Status>>,
    n_rdy: Arc<AtomicBool>,
    last_received_frame_num: Arc<RwLock<Option<WrappingU3>>>,
    buffer: FrameBuffer,
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
        response: Sender<Response>,
        status: Arc<RwLock<Status>>,
        n_rdy: Arc<AtomicBool>,
        last_received_frame_num: Arc<RwLock<Option<WrappingU3>>>,
    ) -> Self {
        Self {
            serial_port,
            response,
            status,
            n_rdy,
            last_received_frame_num,
            buffer: FrameBuffer::new(),
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

    fn status(&self) -> Status {
        *self.status.read().expect("RW lock poisoned")
    }

    fn set_status(&self, status: Status) {
        *self.status.write().expect("RW lock poisoned") = status;
    }

    fn ack_number(&self) -> WrappingU3 {
        self.last_received_frame_num
            .read()
            .expect("RW lock poisoned")
            .map_or_else(WrappingU3::default, |ack_number| ack_number + 1)
    }

    /// Main loop of the transceiver.
    ///
    /// This method checks whether the transceiver is connected and establishes a connection if not.
    /// Otherwise, it will communicate with the NCP via the `ASHv2` protocol.
    fn main(&mut self) -> std::io::Result<()> {
        match self.status() {
            Status::Disconnected | Status::Failed => Ok(self.connect()?),
            Status::Connected => {
                if let Some(packet) = self.receive()? {
                    self.handle_packet(packet)
                } else {
                    Ok(())
                }
            }
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
                            "Received RSTACK is not ASHv2.",
                        ));
                    }

                    self.set_status(Status::Connected);
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
        self.send_ack(&Ack::new(self.ack_number(), self.n_rdy.load(Relaxed)))
    }

    /// Send a `NAK` frame with the current ACK number.
    ///
    /// # Errors
    ///
    /// Returns an [Error] if the serial port read operation failed.
    fn nak(&mut self) -> std::io::Result<()> {
        self.send_nak(&Nak::new(self.ack_number(), self.n_rdy.load(Relaxed)))
    }

    /// Send a RST frame.
    ///
    /// # Errors
    ///
    /// Returns an [Error] if the serial port read operation failed.
    fn rst(&mut self) -> std::io::Result<()> {
        self.serial_port.write_frame(&RST, &mut self.buffer)
    }

    /// Send a raw `ACK` frame.
    fn send_ack(&mut self, ack: &Ack) -> std::io::Result<()> {
        debug!("Sending ACK: {ack}");
        self.serial_port.write_frame(ack, &mut self.buffer)
    }

    /// Send a raw `NAK` frame.
    fn send_nak(&mut self, nak: &Nak) -> std::io::Result<()> {
        debug!("Sending NAK: {nak}");
        self.serial_port.write_frame(nak, &mut self.buffer)
    }

    /// Read an ASH [`Packet`].
    ///
    /// # Errors
    ///
    /// Returns an [`Error`] if any I/O, protocol or parsing error occurs.
    fn read_packet(&mut self) -> std::io::Result<Packet> {
        self.buffer_frame()?;
        Packet::try_from(self.buffer.as_slice())
    }

    /// Reads an ASH frame into the transceiver's frame buffer.
    ///
    /// # Errors
    /// Returns an [`Error`] if any I/O or protocol error occurs.
    fn buffer_frame(&mut self) -> std::io::Result<()> {}
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

        if self.state.status() == Status::Connected {
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
        trace!("Unmasked data: {:#04X}", data.unmasked());

        if !data.is_crc_valid() {
            warn!("Received data frame with invalid CRC.");
            self.enter_reject()?;
        } else if data.frame_num() == self.state.ack_number() {
            self.leave_reject();
            self.state.set_last_received_frame_num(data.frame_num());
            self.ack()?;
            self.ack_sent_packets(data.ack_num());
            self.handle_payload(data.into_payload());
        } else if data.is_retransmission() {
            info!("Received retransmission of frame: {data}");
            self.ack()?;
            self.ack_sent_packets(data.ack_num());
            self.handle_payload(data.into_payload());
        } else {
            warn!("Received out-of-sequence data frame: {data}");
            self.enter_reject()?;
        }

        Ok(())
    }

    /// Extends the response buffer with the given data.
    fn handle_payload(&mut self, mut payload: Payload) {
        payload.mask();
        self.channels.respond(payload);
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

        Error::new(ErrorKind::ConnectionReset, "NCP entered ERROR state.")
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
            "NCP received unexpected RSTACK.",
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
        if self.state.reject() {
            Ok(())
        } else {
            trace!("Entering rejection state.");
            self.state.set_reject(true);
            self.nak()
        }
    }

    /// Leave the rejection state.
    fn leave_reject(&mut self) {
        if self.state.reject() {
            trace!("Leaving rejection state.");
            self.state.set_reject(false);
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
        self.channels.close();
        self.buffers.clear();
        self.state.reset(Status::Failed);
    }

    /// Handle I/O errors.
    fn handle_io_error(&mut self, error: &Error) {
        error!("{error}");

        if self.state.within_transaction() {
            error!("Aborting current transaction due to error.");
        }

        self.reset();
    }
}
