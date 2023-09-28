use super::{CANCEL, FLAG, SUBSTITUTE, TIMEOUT, X_OFF, X_ON};
use crate::packet::ack::Ack;
use crate::packet::data::Data;
use crate::packet::error::Error;
use crate::packet::nak::Nak;
use crate::packet::rst_ack::RstAck;
use crate::packet::Packet;
use crate::protocol::stuffing::Stuffing;
use anyhow::anyhow;
use log::debug;
use serialport::SerialPort;
use std::cell::{Ref, RefCell};
use std::collections::HashMap;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering::SeqCst;

const MAX_BUF_CAPACITY: usize = 132 * 2; // Worst case: every byte is escaped.
type Subscriber = fn(&[u8]) -> Option<Vec<u8>>;

pub struct Host<S>
where
    S: SerialPort,
{
    listener: RefCell<Listener<S>>,
    byte_buffer: RefCell<[u8; 1]>,
    close: AtomicBool,
    subscribers: Vec<Subscriber>,
    queue: HashMap<u8, Option<Vec<u8>>>,
}

impl<S> Host<S>
where
    S: SerialPort,
{
    pub fn new(serial_port: S) -> Self {
        Self {
            listener: RefCell::new(Listener::new(serial_port)),
            byte_buffer: RefCell::new([0]),
            close: AtomicBool::new(false),
            subscribers: Vec::new(),
            queue: HashMap::new(),
        }
    }

    async fn send(&mut self, bytes: &[u8]) -> Vec<u8> {
        todo!()
    }
}

struct Listener<S>
where
    S: SerialPort,
{
    serial_port: S,
    buffer: [u8; 1],
    close: AtomicBool,
    ack_num: u8,
}

impl<S> Listener<S>
where
    S: SerialPort,
{
    pub fn new(serial_port: S) -> Self {
        Self {
            serial_port,
            buffer: [0],
            close: AtomicBool::new(false),
            ack_num: 0,
        }
    }

    fn listen(mut self) {
        let mut errors: usize = 0;
        let mut reject = false;

        while !self.close.load(SeqCst) {
            let response: Packet;

            match self.read_packet() {
                Ok(packet) => {
                    debug!("RX ASH frame: {packet}");
                    errors = 0;

                    match packet {
                        Packet::Data(data) => self.handle_data(data),
                        Packet::Ack(ack) => self.handle_ack(ack),
                        Packet::Nak(nak) => self.handle_nak(nak),
                        Packet::RstAck(rst_ack) => self.handle_rst_ack(rst_ack),
                        Packet::Error(error) => self.handle_error(error),
                        packet @ Packet::Rst(_) => {
                            debug!("Ignoring packet: {packet}");
                        }
                    }
                }
                Err(error) => {
                    debug!("Bad packet: {error}");

                    if !reject {
                        reject = true;
                        response = Packet::Nak(self.ack_num.into());
                    }
                }
            }
        }
    }

    pub fn read_packet(&mut self) -> anyhow::Result<Packet> {
        // TODO: Perform unstuffing before try_from() call!
        Ok(Packet::try_from(self.read_frame()?.as_slice())?)
    }

    fn read_frame(&mut self) -> anyhow::Result<Vec<u8>> {
        let mut buffer = Vec::with_capacity(MAX_BUF_CAPACITY);
        let mut skip_to_next_flag = false;

        while !self.close.load(SeqCst) {
            match self.read_byte()? {
                CANCEL => {
                    buffer.clear();
                    skip_to_next_flag = false;
                }
                FLAG => {
                    if !skip_to_next_flag && !buffer.is_empty() {
                        return Ok(buffer.into_iter().unstuff().collect());
                    }

                    buffer.clear();
                    skip_to_next_flag = false;
                }
                SUBSTITUTE => {
                    buffer.clear();
                    skip_to_next_flag = true;
                }
                X_ON | X_OFF | TIMEOUT => continue,
                byte => {
                    if buffer.len() > MAX_BUF_CAPACITY {
                        buffer.clear();
                        skip_to_next_flag = true;
                    }

                    buffer.push(byte);
                }
            }
        }

        Err(anyhow!("Reading aborted."))
    }

    fn read_byte(&mut self) -> std::io::Result<u8> {
        self.serial_port.read_exact(&mut self.buffer)?;
        Ok(self.buffer[0])
    }

    fn handle_data(&self, data: Data) {
        todo!()
    }

    fn handle_ack(&self, ack: Ack) {
        todo!()
    }

    fn handle_nak(&self, nak: Nak) {
        todo!()
    }

    fn handle_rst_ack(&self, rst_ack: RstAck) {
        todo!()
    }

    fn handle_error(&self, error: Error) {
        todo!()
    }
}
