use super::{CANCEL, FLAG, SUBSTITUTE, TIMEOUT, X_OFF, X_ON};
use crate::packet::ack::Ack;
use crate::packet::data::Data;
use crate::packet::error::Error;
use crate::packet::nak::Nak;
use crate::packet::rst_ack::RstAck;
use crate::packet::Packet;
use anyhow::anyhow;
use log::debug;
use serialport::SerialPort;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering::SeqCst;

const MAX_BUF_CAPACITY: usize = 124;

pub struct Host<S>
where
    S: SerialPort,
{
    serial_port: S,
    byte_buffer: [u8; 1],
    close: AtomicBool,
}

impl<S> Host<S>
where
    S: SerialPort,
{
    pub const fn new(serial_port: S) -> Self {
        Self {
            serial_port,
            byte_buffer: [0],
            close: AtomicBool::new(false),
        }
    }

    fn run(&mut self) {
        let mut errors: usize = 0;
        let mut reject = false;
        let mut ack_num: u8 = 0;

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
                        response = Packet::Nak(ack_num.into());
                    }
                }
            }
        }
    }

    fn read_packet(&mut self) -> anyhow::Result<Packet> {
        // TODO: Perform unstuffing before try_from() call!
        Ok(Packet::try_from(self.read_frame()?.as_slice())?)
    }

    fn read_frame(&mut self) -> anyhow::Result<Vec<u8>> {
        let mut buffer = Vec::with_capacity(MAX_BUF_CAPACITY);
        let mut skip_to_next_flag = false;

        while !self.close.load(SeqCst) {
            self.serial_port.read_exact(&mut self.byte_buffer)?;

            match self.byte_buffer[0] {
                CANCEL => {
                    buffer.clear();
                    skip_to_next_flag = false;
                }
                FLAG => {
                    if !skip_to_next_flag && !buffer.is_empty() {
                        buffer.push(FLAG);
                        return Ok(buffer);
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

    fn handle_data(&mut self, data: Data) {
        todo!()
    }

    fn handle_ack(&mut self, ack: Ack) {
        todo!()
    }

    fn handle_nak(&mut self, nak: Nak) {
        todo!()
    }

    fn handle_rst_ack(&mut self, rst_ack: RstAck) {
        todo!()
    }

    fn handle_error(&mut self, error: Error) {
        todo!()
    }
}
