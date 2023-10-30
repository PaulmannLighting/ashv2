use crate::packet::Packet;
use crate::protocol::{Stuffing, CANCEL, FLAG, SUBSTITUTE, WAKE, X_OFF, X_ON};
use crate::Error;
use log::{debug, error, info, trace};
use serialport::SerialPort;
use std::fmt::Debug;
use std::io::{ErrorKind, Read};
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering::SeqCst;
use std::sync::mpsc::Sender;
use std::sync::Arc;

#[derive(Debug)]
pub struct AshReceiver {
    // Shared state
    serial_port: Box<dyn SerialPort>,
    terminate: Arc<AtomicBool>,
    packets: Sender<Result<Packet, Error>>,
    may_transmit: Arc<AtomicBool>,
    // Local state
    buffer: Vec<u8>,
}

impl AshReceiver {
    pub fn new(
        serial_port: Box<dyn SerialPort>,
        terminate: Arc<AtomicBool>,
        packets: Sender<Result<Packet, Error>>,
        may_transmit: Arc<AtomicBool>,
    ) -> Self {
        Self {
            serial_port,
            terminate,
            packets,
            may_transmit,
            buffer: Vec::new(),
        }
    }

    pub fn spawn(mut self) {
        while !self.terminate.load(SeqCst) {
            let packet = self.receive_packet();
            self.packets.send(packet).unwrap_or_else(|error| {
                error!("Could not send received packet.");
                debug!("{error}");
            });
        }
    }

    fn receive_packet(&mut self) -> Result<Packet, Error> {
        Ok(Packet::try_from(self.receive_frame()?.as_slice())?)
    }

    fn receive_frame(&mut self) -> Result<Vec<u8>, Error> {
        self.buffer.clear();
        let mut error = false;

        for byte in (&mut self.serial_port).bytes() {
            match byte? {
                CANCEL => {
                    debug!("Resetting buffer due to cancel byte.");
                    trace!("Error condition: {error}");
                    trace!("Buffer content: {:#04X?}", self.buffer);
                    self.buffer.clear();
                    error = false;
                }
                FLAG => {
                    if !error && !self.buffer.is_empty() {
                        debug!("Received frame.");
                        trace!("Frame details: {:#04X?}", self.buffer);
                        return Ok(self.buffer.iter().copied().unstuff().collect());
                    }

                    debug!("Resetting buffer due to error or empty buffer.");
                    trace!("Error condition: {error}");
                    trace!("Buffer content: {:#04X?}", self.buffer);
                    self.buffer.clear();
                    error = false;
                }
                SUBSTITUTE => {
                    debug!("Received SUBSTITUTE byte. Setting error condition.");
                    error = true;
                }
                X_ON => {
                    info!("NCP requested to stop transmission.");
                    self.may_transmit.store(true, SeqCst);
                }
                X_OFF => {
                    info!("NCP requested to resume transmission.");
                    self.may_transmit.store(false, SeqCst);
                }
                WAKE => {
                    info!("NCP tried to wake us up.");
                }
                byte => self.buffer.push(byte),
            }
        }

        Err(Error::Io(std::io::Error::new(
            ErrorKind::UnexpectedEof,
            "No more bytes to read.",
        )))
    }
}
