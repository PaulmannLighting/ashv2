use std::fmt::{LowerHex, UpperHex};
use std::io::{Error, ErrorKind, Read};
use std::sync::{Arc, Mutex, MutexGuard, RwLock, RwLockReadGuard, RwLockWriteGuard};
use std::time::SystemTime;

use log::{debug, error, info, trace, warn};
use serialport::SerialPort;
use tokio::sync::mpsc::Sender;

use crate::frame::Frame;
use crate::packet::{Ack, Data, Nak, Packet, RstAck, RST};
use crate::protocol::{Mask, Stuffing, CANCEL, FLAG, SUBSTITUTE, WAKE, X_OFF, X_ON};
use crate::response::Response;
use crate::shared_state::SharedState;
use crate::status::Status;
use crate::types::FrameBuffer;
use crate::write_frame::WriteFrame;
use crate::{HexSlice, Payload};

/// `ASHv2` transceiver.
///
/// The transceiver is responsible for handling the communication between the host and the NCP.
///
/// It is supposed to be run in a separate thread.
///
/// The [`AshFramed`](crate::AshFramed) struct implements a stream
/// to communicate with the NCP via the transceiver.
#[derive(Debug)]
pub struct Receiver<T>
where
    T: SerialPort,
{
    serial_port: T,
    responses: Arc<Mutex<Option<Sender<Response>>>>,
    callbacks: Option<Sender<Payload>>,
    buffer: FrameBuffer,
    state: Arc<RwLock<SharedState>>,
}

impl<T> Receiver<T>
where
    T: SerialPort,
{
    /// Create a new receiver.
    ///
    /// # Parameters
    ///
    /// - `serial_port`: The serial port to communicate with the NCP.
    /// - `responses`: The channel to send responses to commands through.
    /// - `callback`: An optional channel to send callbacks from the NCP through.
    ///
    /// If no callback channel is provided, the receiver will
    /// discard any callbacks actively sent from the NCP.
    #[must_use]
    pub const fn new(
        serial_port: T,
        state: Arc<RwLock<SharedState>>,
        responses: Arc<Mutex<Option<Sender<Response>>>>,
        callbacks: Option<Sender<Payload>>,
    ) -> Self {
        Self {
            serial_port,
            responses,
            callbacks,
            buffer: FrameBuffer::new(),
            state,
        }
    }

    /// Run the transceiver.
    ///
    /// This should be called in a separate thread.
    pub async fn run(mut self) {
        loop {
            if let Err(error) = self.main().await {
                error!("{error}");

                if self.responses().is_some() {
                    error!("Aborting current transaction due to error.");
                }

                self.reset();
            }
        }
    }

    /// Main loop of the transceiver.
    ///
    /// This method checks whether the transceiver is connected and establishes a connection if not.
    /// Otherwise, it will communicate with the NCP via the `ASHv2` protocol.
    async fn main(&mut self) -> std::io::Result<()> {
        let status = self.state.read().expect("RW lock poisoned").status();

        match status {
            Status::Disconnected | Status::Failed => Ok(self.connect()?),
            Status::Connected => {
                let packet = self.receive()?;
                self.handle_packet(packet).await
            }
        }
    }

    /// Establish an `ASHv2` connection with the NCP.
    fn connect(&mut self) -> std::io::Result<()> {
        debug!("Connecting to NCP...");
        let start = SystemTime::now();
        let mut attempts: usize = 0;

        loop {
            attempts += 1;
            self.rst()?;

            debug!("Waiting for RSTACK...");

            match self.receive()? {
                Packet::RstAck(rst_ack) => {
                    if !rst_ack.is_ash_v2() {
                        return Err(Error::new(
                            ErrorKind::Unsupported,
                            "Received RSTACK is not ASHv2.",
                        ));
                    }

                    self.state_mut().set_status(Status::Connected);
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

    /// Read an ASH [`Packet`].
    ///
    /// # Errors
    ///
    /// Returns an [`Error`] if any I/O, protocol or parsing error occurs.
    fn receive(&mut self) -> std::io::Result<Packet> {
        Packet::try_from(self.buffer_frame()?)
    }

    /// Reads an ASH frame into the transceiver's frame buffer.
    ///
    /// # Errors
    /// Returns an [`Error`] if any I/O or protocol error occurs.
    fn buffer_frame(&mut self) -> std::io::Result<&[u8]> {
        self.buffer.clear();
        let mut error = false;

        for byte in (&mut self.serial_port).bytes() {
            match byte? {
                CANCEL => {
                    trace!("Resetting buffer due to cancel byte.");
                    self.buffer.clear();
                    error = false;
                }
                FLAG => {
                    trace!("Received flag byte.");

                    if !error && !self.buffer.is_empty() {
                        debug!("Received frame.");
                        trace!("Buffer: {:#04X}", HexSlice::new(&self.buffer));
                        self.buffer.unstuff();
                        trace!("Unstuffed buffer: {:#04X}", HexSlice::new(&self.buffer));
                        return Ok(&self.buffer);
                    }

                    trace!("Resetting buffer due to error or empty buffer.");
                    trace!("Error condition was: {error}");
                    trace!("Buffer: {:#04X}", HexSlice::new(&self.buffer));
                    self.buffer.clear();
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
                    if self.buffer.is_empty() {
                        debug!("NCP tried to wake us up.");
                    } else if self.buffer.push(WAKE).is_err() {
                        return Err(Error::new(ErrorKind::OutOfMemory, "Frame buffer overflow."));
                    }
                }
                byte => {
                    if self.buffer.push(byte).is_err() {
                        return Err(Error::new(ErrorKind::OutOfMemory, "Frame buffer overflow."));
                    }
                }
            }
        }

        Err(Error::new(
            ErrorKind::UnexpectedEof,
            "Byte stream terminated unexpectedly.",
        ))
    }

    /// Handle an incoming packet.
    ///
    /// # Errors
    ///
    /// Returns a [Error] if the packet handling failed.
    async fn handle_packet(&mut self, packet: Packet) -> std::io::Result<()> {
        debug!("Handling: {packet}");
        trace!("{packet:#04X}");

        if self.state().status() == Status::Connected {
            match packet {
                Packet::Ack(ref ack) => self.forward(Response::Ack(ack.ack_num())).await,
                Packet::Data(data) => self.handle_data(data).await?,
                Packet::Error(ref error) => return Err(Self::handle_error(error)),
                Packet::Nak(ref nak) => self.forward(Response::Nak(nak.ack_num())).await,
                Packet::RstAck(ref rst_ack) => return Err(Self::handle_rst_ack(rst_ack)),
                Packet::Rst(_) => warn!("Received unexpected RST from NCP."),
            }
        } else {
            warn!("Not connected. Dropping frame: {packet}");
        }

        Ok(())
    }

    /// Handle an incoming `DATA` packet.
    async fn handle_data(&mut self, data: Data) -> std::io::Result<()> {
        trace!("Unmasked data: {:#04X}", data.unmasked());

        if !data.is_crc_valid() {
            warn!("Received data frame with invalid CRC.");
            self.state_mut().set_reject(true);
            self.nak()?;
        } else if data.frame_num() == self.state().ack_number() {
            trace!("Leaving rejection state.");
            self.state_mut()
                .set_last_received_frame_num(data.frame_num());

            if self.state().frame_number() == data.ack_num() {
                self.ack()?;
            } else {
                debug!("Waiting for further data before acknowledging.");
            }

            self.forward_payload(data.into_payload()).await;
        } else if data.is_retransmission() {
            info!("Received retransmission of frame: {data}");
            self.ack()?;
            self.forward_payload(data.into_payload()).await;
        } else {
            warn!("Received out-of-sequence data frame: {data}");
            self.state_mut().set_reject(true);
            self.nak()?;
        }

        Ok(())
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

    /// Extends the response buffer with the given data.
    async fn forward_payload(&mut self, mut payload: Payload) {
        payload.mask();
        self.forward(Response::Data(payload)).await;
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

    async fn forward(&self, response: Response) {
        if let Some(response) = self.try_respond(response).await {
            if let Response::Data(data) = response {
                if let Some(callbacks) = self.callbacks.as_ref() {
                    if let Err(_) = callbacks.send(data).await {
                        error!("Failed to send response. Closing callback channel forever.");
                    } else {
                        warn!("No response channel available. Discarding data packet.");
                    }
                }
            } else {
                warn!("No response channel available. Discarding packet: {response:?}");
            }
        }
    }

    async fn try_respond(&self, response: Response) -> Option<Response> {
        let mut lock = self.responses();

        if let Some(responses) = lock.as_ref() {
            if let Err(_) = responses.send(response).await {
                error!("Failed to send response. Closing response channel.");
                lock.take();
            }

            return None;
        }

        Some(response)
    }

    /// Send an `ACK` frame with the given ACK number.
    ///
    /// # Errors
    ///
    /// Returns an [Error] if the serial port read operation failed.
    fn ack(&mut self) -> std::io::Result<()> {
        let ack = Ack::new(self.state().ack_number(), self.responses().is_some());
        self.send_ack(&ack)
    }

    /// Send a raw `ACK` frame.
    fn send_ack(&mut self, ack: &Ack) -> std::io::Result<()> {
        if ack.not_ready() {
            self.state_mut()
                .set_last_n_rdy_transmission(SystemTime::now());
        }

        debug!("Sending ACK: {ack}");
        self.serial_port.write_frame(ack, &mut self.buffer)
    }

    /// Send a `NAK` frame with the current ACK number.
    ///
    /// # Errors
    ///
    /// Returns an [Error] if the serial port read operation failed.
    fn nak(&mut self) -> std::io::Result<()> {
        let nak = Nak::new(self.state().ack_number(), self.responses().is_some());
        self.send_nak(&nak)
    }

    /// Send a raw `NAK` frame.
    fn send_nak(&mut self, nak: &Nak) -> std::io::Result<()> {
        if nak.not_ready() {
            self.state_mut()
                .set_last_n_rdy_transmission(SystemTime::now());
        }

        debug!("Sending NAK: {nak}");
        self.serial_port.write_frame(nak, &mut self.buffer)
    }

    /// Send a RST frame.
    ///
    /// # Errors
    ///
    /// Returns an [Error] if the serial port read operation failed.
    fn rst(&mut self) -> std::io::Result<()> {
        self.serial_port.write_frame(&RST, &mut self.buffer)
    }

    fn state(&self) -> RwLockReadGuard<'_, SharedState> {
        self.state.read().expect("RW lock poisoned")
    }

    fn state_mut(&self) -> RwLockWriteGuard<'_, SharedState> {
        self.state.write().expect("RW lock poisoned")
    }

    fn responses(&self) -> MutexGuard<'_, Option<Sender<Response>>> {
        self.responses.lock().expect("mutex poisoned")
    }

    /// Reset buffers and state.
    fn reset(&mut self) {
        self.responses().take();
        self.buffer.clear();
        self.state_mut().reset(Status::Failed);
    }
}
