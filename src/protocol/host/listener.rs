use crate::frame::Frame;
use crate::packet::{Ack, Data, Error, FrameBuffer, Nak, Packet, RstAck};
use crate::protocol::host::command::{Command, Event, HandleResult, Response};
use crate::protocol::Mask;
use crate::util::next_three_bit_number;
use crate::{AshRead, AshWrite};
use itertools::Itertools;
use log::{debug, error, info, trace, warn};
use serialport::SerialPort;
use std::fmt::Debug;
use std::io::ErrorKind;
use std::sync::atomic::Ordering::SeqCst;
use std::sync::atomic::{AtomicBool, AtomicU8};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::{Arc, Mutex, RwLock, RwLockReadGuard, RwLockWriteGuard};

#[derive(Debug)]
pub struct Listener<S>
where
    S: SerialPort,
{
    // Shared state
    serial_port: Arc<Mutex<S>>,
    running: Arc<AtomicBool>,
    connected: Arc<AtomicBool>,
    current_command: Arc<RwLock<Option<Command>>>,
    ack_number: Arc<AtomicU8>,
    callback: Option<Sender<Arc<[u8]>>>,
    ack_sender: Sender<u8>,
    nak_sender: Sender<u8>,
    // Local state
    read_buffer: FrameBuffer,
    write_buffer: FrameBuffer,
    is_rejecting: bool,
    last_received_frame_number: Option<u8>,
}

impl<S> Listener<S>
where
    S: SerialPort,
{
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        serial_port: Arc<Mutex<S>>,
        running: Arc<AtomicBool>,
        connected: Arc<AtomicBool>,
        current_command: Arc<RwLock<Option<Command>>>,
        ack_number: Arc<AtomicU8>,
        callback: Option<Sender<Arc<[u8]>>>,
        ack_sender: Sender<u8>,
        nak_sender: Sender<u8>,
    ) -> Self {
        Self {
            serial_port,
            running,
            connected,
            current_command,
            ack_number,
            callback,
            ack_sender,
            nak_sender,
            read_buffer: FrameBuffer::new(),
            write_buffer: FrameBuffer::new(),
            is_rejecting: false,
            last_received_frame_number: None,
        }
    }

    pub fn create(
        serial_port: Arc<Mutex<S>>,
        running: Arc<AtomicBool>,
        connected: Arc<AtomicBool>,
        current_command: Arc<RwLock<Option<Command>>>,
        ack_number: Arc<AtomicU8>,
        callback: Option<Sender<Arc<[u8]>>>,
    ) -> (Self, Receiver<u8>, Receiver<u8>) {
        let (ack_sender, ack_receiver) = channel();
        let (nak_sender, nak_receiver) = channel();
        let listener = Self::new(
            serial_port,
            running,
            connected,
            current_command,
            ack_number,
            callback,
            ack_sender,
            nak_sender,
        );
        (listener, ack_receiver, nak_receiver)
    }

    pub fn run(mut self) -> Option<Sender<Arc<[u8]>>> {
        while self.running.load(SeqCst) {
            match self.read_frame() {
                Ok(packet) => {
                    if let Some(ref frame) = packet {
                        self.handle_frame(frame);
                    }
                }
                Err(error) => error!("{error}"),
            }
        }

        info!("Terminating.");
        self.callback
    }

    fn handle_frame(&mut self, frame: &Packet) {
        debug!("Received: {frame}");
        trace!("{frame:#04X?}");

        if self.connected.load(SeqCst) {
            match frame {
                Packet::Ack(ref ack) => self.handle_ack(ack),
                Packet::Data(ref data) => self.handle_data(data),
                Packet::Error(ref error) => self.handle_error(error),
                Packet::Nak(ref nak) => self.handle_nak(nak),
                Packet::RstAck(ref rst_ack) => self.handle_rst_ack(rst_ack),
                Packet::Rst(_) => warn!("Received unexpected RST from NCP."),
            }
        } else if let Packet::RstAck(ref rst_ack) = frame {
            self.handle_rst_ack(rst_ack);
        } else {
            warn!("Not connected. Dropping frame: {frame}");
        }
    }

    fn handle_ack(&mut self, ack: &Ack) {
        if !ack.is_crc_valid() {
            warn!("Received ACK with invalid CRC.");
        }

        self.ack_sender
            .send(ack.ack_num())
            .unwrap_or_else(|error| error!("Failed to forward ACK: {error}"));
    }

    fn handle_data(&mut self, data: &Data) {
        debug!("Received frame: {data}");
        trace!("Frame details: {data:#04X?}");
        trace!(
            "Unmasked payload: {:#04X?}",
            data.payload().iter().copied().mask().collect_vec()
        );

        if !data.is_crc_valid() {
            warn!("Received data frame with invalid CRC.");
            self.reject();
        } else if data.frame_num() == self.ack_number() {
            self.ack_received_data(data.frame_num());
            self.is_rejecting = false;
            self.last_received_frame_number = Some(data.frame_num());
            self.ack_number.store(self.ack_number(), SeqCst);
            debug!("Sending ACK to transmitter: {}", data.ack_num());
            self.ack_sender
                .send(data.ack_num())
                .unwrap_or_else(|error| {
                    error!("Failed to forward ACK: {error}");
                });
            self.forward_data(data);
        } else if data.is_retransmission() {
            self.ack_number.store(self.ack_number(), SeqCst);
            debug!("Sending ACK to transmitter: {}", data.ack_num());
            self.ack_sender
                .send(data.ack_num())
                .unwrap_or_else(|error| {
                    error!("Failed to forward ACK: {error}");
                });
            self.forward_data(data);
        } else {
            debug!("Received out-of-sequence data frame: {data}");

            if !self.is_rejecting {
                self.reject();
            }
        }
    }

    fn ack_received_data(&mut self, frame_num: u8) {
        self.write_frame(&Ack::from_ack_num(next_three_bit_number(frame_num)))
            .unwrap_or_else(|error| error!("Failed to send ACK: {error}"));
    }

    fn forward_data(&mut self, data: &Data) {
        debug!("Forwarding data: {data}");
        let payload: Arc<[u8]> = data.payload().iter().copied().mask().collect_vec().into();

        if let Some(command) = self.current_command().as_ref() {
            if let Command::Data(_, response) = command {
                debug!("Forwarding data to current command.");
                match response.handle(Event::DataReceived(Ok(payload.clone()))) {
                    HandleResult::Completed => {
                        debug!("Command responded with COMPLETED.");
                        response.wake();
                        self.current_command_mut().take();
                    }
                    HandleResult::Continue => debug!("Command responded with CONTINUE."),
                    HandleResult::Failed => {
                        warn!("Command responded with FAILED.");
                        response.wake();
                        self.current_command_mut().take();
                    }
                    HandleResult::Reject => {
                        debug!("Command responded with REJECT.");
                        self.callback.as_ref().map_or_else(|| {
                            error!("Current response handler rejected received data and there is no callback handler registered. Dropping packet.");
                        }, |callback| {
                            debug!("Forwarding rejected data to callback.");
                            callback.send(payload).unwrap_or_else(|error| {
                                error!("Failed to send data to callback channel: {error}");
                            });
                        });
                    }
                };
            } else if let Some(callback) = &self.callback {
                debug!("Forwarding data to callback.");
                callback.send(payload).unwrap_or_else(|error| {
                    error!("Failed to send data to callback channel: {error}");
                });
            } else {
                error!("There is neither an active response handler nor a callback handler registered. Dropping packet.");
            }
        }
    }

    fn handle_error(&mut self, error: &Error) {
        trace!("Received ERROR: {error:#04X?}");
        self.connected.store(false, SeqCst);
        error.code().map_or_else(
            || {
                error!("NCP sent error without valid code.");
            },
            |code| {
                warn!("NCP sent error condition: {code}");
            },
        );
    }

    fn handle_nak(&mut self, nak: &Nak) {
        if !nak.is_crc_valid() {
            warn!("Received ACK with invalid CRC.");
        }

        debug!("Forwarding NAK to transmitter.");
        self.nak_sender
            .send(nak.ack_num())
            .unwrap_or_else(|error| error!("Failed to forward NAK: {error}"));
    }

    fn handle_rst_ack(&mut self, rst_ack: &RstAck) {
        rst_ack.code().map_or_else(
            || {
                warn!("NCP acknowledged reset with invalid error code.");
            },
            |code| {
                debug!("NCP acknowledged reset due to: {code}");
            },
        );
        self.reset_state();
        self.connected.store(true, SeqCst);

        if let Some(Command::Reset(reset_response)) = self.current_command().as_ref() {
            match reset_response.handle(Event::DataReceived(Ok(()))) {
                HandleResult::Completed | HandleResult::Failed => {
                    reset_response.wake();
                    self.current_command_mut().take();
                }
                HandleResult::Continue => (),
                HandleResult::Reject => warn!("Reset response handler rejected our reset."),
            }
        }
    }

    fn reset_state(&mut self) {
        trace!("Resetting state variables.");
        self.read_buffer.clear();
        self.is_rejecting = false;
        self.last_received_frame_number = None;
    }

    fn reject(&mut self) {
        trace!("Entering rejection state.");
        self.is_rejecting = true;
        self.send_nak();
    }

    fn send_nak(&mut self) {
        debug!("Sending NAK: {}", self.ack_number());
        self.write_frame(&Nak::from_ack_num(self.ack_number()))
            .unwrap_or_else(|error| error!("Could not send NAK: {error}"));
    }

    fn read_frame(&mut self) -> Result<Option<Packet>, crate::Error> {
        self.serial_port
            .lock()
            .expect("Failed to lock serial port.")
            .read_frame(&mut self.read_buffer)
            .map(Some)
            .or_else(|error| {
                if let crate::Error::Io(io_error) = &error {
                    if io_error.kind() == ErrorKind::TimedOut {
                        return Ok(None);
                    }
                }
                Err(error)
            })
    }

    fn write_frame<F>(&mut self, frame: &F) -> std::io::Result<()>
    where
        F: Frame,
        for<'a> &'a F: IntoIterator<Item = u8>,
    {
        self.serial_port
            .lock()
            .expect("Failed to lock serial port.")
            .write_frame(frame, &mut self.write_buffer)
    }

    fn current_command(&self) -> RwLockReadGuard<'_, Option<Command>> {
        trace!("Retrieving current command.");
        self.current_command
            .read()
            .expect("Failed to lock current command.")
    }

    fn current_command_mut(&self) -> RwLockWriteGuard<'_, Option<Command>> {
        trace!("Retrieving current command.");
        self.current_command
            .write()
            .expect("Failed to lock current command.")
    }

    const fn ack_number(&self) -> u8 {
        if let Some(last_received_frame_number) = self.last_received_frame_number {
            next_three_bit_number(last_received_frame_number)
        } else {
            0
        }
    }
}
