use super::buffers::Buffers;
use super::state::State;
use crate::frame::Frame;
use crate::packet::{Data, Packet};
use crate::protocol::host::transaction::Transaction;
use crate::protocol::{AshChunks, Mask, Stuffing, CANCEL, FLAG, SUBSTITUTE, TIMEOUT, X_OFF, X_ON};
use log::{debug, info, trace, warn};
use serialport::SerialPort;
use std::io::Read;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::SystemTime;

#[derive(Debug)]
pub struct Receiver<S>
where
    S: SerialPort,
{
    // Shared state
    serial_port: Arc<Mutex<S>>,
    transaction: Arc<Mutex<Transaction>>,
    terminate: Arc<Mutex<AtomicBool>>,
    buffers: Arc<Mutex<Buffers>>,
    state: Arc<Mutex<State>>,
    may_transmit: Arc<Mutex<AtomicBool>>,
    // Local state
    data: Vec<(SystemTime, Data)>,
    buffer: Vec<u8>,
    byte: [u8; 1],
}

impl<S> Receiver<S>
where
    S: SerialPort,
{
    pub fn new(
        serial_port: Arc<Mutex<S>>,
        transaction: Arc<Mutex<Transaction>>,
        terminate: Arc<Mutex<AtomicBool>>,
        buffers: Arc<Mutex<Buffers>>,
        state: Arc<Mutex<State>>,
        may_transmit: Arc<Mutex<AtomicBool>>,
    ) -> Self {
        Self {
            serial_port,
            transaction,
            terminate,
            buffers,
            state,
            may_transmit: may_transmit,
            data: Vec::new(),
            buffer: Vec::new(),
            byte: [0],
        }
    }

    fn spawn(mut self) {
        while self.is_running() {
            match self.receive_packet() {
                Ok(frame) => todo!(),
                Err(error) => todo!(),
            }
        }
    }

    fn receive_and_process_packet(&mut self) -> Result<(), crate::Error> {
        debug!("Receiving packet.");

        if let Some(packet) = self.receive_packet()? {
            match packet {
                Packet::Ack(ref ack) => self.process_ack(ack)?,
                Packet::Data(data) => self.process_data(data)?,
                Packet::Error(ref error) => self.handle_error(error)?,
                Packet::Nak(ref nak) => self.process_nak(nak),
                Packet::Rst(ref rst) => {
                    error!("NCP sent us an unexpected RST: {rst}");
                    trace!("Frame details: {rst:#04X?}");
                }
                Packet::RstAck(ref rst_ack) => self.process_rst_ack(rst_ack),
            }
        } else {
            self.queue_nak()?;
        }

        Ok(())
    }

    fn receive_packet(&mut self) -> Result<Option<Packet>, crate::Error> {
        Packet::try_from(self.receive_frame()?.as_slice())
            .map(|packet| {
                if packet.is_crc_valid() {
                    Some(packet)
                } else {
                    warn!("Dropping packet with invalid CRC: {packet}");
                    trace!("Raw packet: {packet:#04X?}");
                    None
                }
            })
            .map_err(crate::Error::Frame)
    }

    fn receive_frame(&mut self) -> Result<Vec<u8>, crate::Error> {
        self.buffer.clear();
        let serial_port = self.serial_port.clone();
        let mut reader = serial_port
            .lock()
            .expect("Could not lock serial port for reading.");
        let mut error = false;

        while self.is_running() {
            match self.read_byte(&mut *reader)? {
                CANCEL => {
                    self.buffer.clear();
                    error = false;
                }
                FLAG => {
                    if !error && !self.buffer.is_empty() {
                        debug!("Received frame.");
                        trace!("Raw frame: {:#04X?}", self.buffer);
                        return Ok(self.buffer.iter().cloned().unstuff().collect());
                    }

                    self.buffer.clear();
                    error = false;
                }
                SUBSTITUTE => {
                    error = true;
                }
                X_ON => {
                    info!("NCP requested to stop transmission.");
                    self.set_may_transmit(true);
                }
                X_OFF => {
                    info!("NCP requested to resume transmission.");
                    self.set_may_transmit(false);
                }
                TIMEOUT => {
                    warn!("Received timeout byte not specified in protocol definition.");
                }
                byte => self.buffer.push(byte),
            }
        }

        Err(crate::Error::Terminated)
    }

    fn is_running(&self) -> bool {
        !self
            .terminate
            .lock()
            .expect("Could not lock terminate flag.")
            .load(Ordering::SeqCst)
    }

    fn read_byte<R>(&mut self, reader: &mut R) -> std::io::Result<u8>
    where
        R: Read,
    {
        reader.read_exact(&mut self.byte)?;
        Ok(self.byte[0])
    }

    fn set_may_transmit(&self, status: bool) {
        self.may_transmit
            .lock()
            .expect("Could not lock transmitting flag")
            .store(status, Ordering::SeqCst);
    }
}
