use super::{CANCEL, FLAG, SUBSTITUTE, TIMEOUT, X_OFF, X_ON};
use crate::packet::Packet;
use serialport::SerialPort;
use std::io::Error;
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

    fn read_packet(&mut self) -> anyhow::Result<Packet> {
        Ok(Packet::try_from(self.read_frame()?.as_slice())?)
    }

    fn read_frame(&mut self) -> Result<Vec<u8>, Error> {
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
                X_ON | X_OFF | TIMEOUT => (),
                byte => {
                    if buffer.len() > MAX_BUF_CAPACITY {
                        buffer.clear();
                        skip_to_next_flag = true;
                    }

                    buffer.push(byte);
                }
            }
        }

        Err(Error::last_os_error())
    }
}
