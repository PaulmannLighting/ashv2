use crate::frame::Frame;
use crate::packet::Packet;
use crate::protocol::{Stuffing, FLAG};
use log::{debug, error};
use serialport::SerialPort;
use std::fmt::{Debug, Display};
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering::SeqCst;
use std::sync::mpsc::Receiver;
use std::sync::Arc;

#[derive(Debug)]
pub struct AshSender {
    // Shared state
    serial_port: Box<dyn SerialPort>,
    receiver: Receiver<Packet>,
    terminate: Arc<AtomicBool>,
    // Local state
    buffer: Vec<u8>,
}

impl AshSender {
    pub fn new(
        serial_port: Box<dyn SerialPort>,
        receiver: Receiver<Packet>,
        terminate: Arc<AtomicBool>,
    ) -> Self {
        Self {
            serial_port,
            receiver,
            terminate,
            buffer: Vec::new(),
        }
    }

    pub fn spawn(mut self) {
        while !self.terminate.load(SeqCst) {
            match self.receiver.recv() {
                Ok(ref packet) => {
                    if let Err(error) = self.send_packet(packet) {
                        error!("{error}");
                    }
                }
                Err(error) => {
                    error!("Failed to receive packet request.");
                    debug!("{error}");
                }
            }
        }
    }

    fn send_packet<P>(&mut self, packet: &P) -> std::io::Result<()>
    where
        P: Debug + Display + Frame,
        for<'a> &'a P: IntoIterator<Item = u8>,
    {
        self.buffer.clear();
        self.buffer.extend(packet.into_iter().stuff());
        self.buffer.push(FLAG);
        self.serial_port.write_all(&mut self.buffer)
    }
}
