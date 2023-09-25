use crate::packet::Packet;
use serialport::SerialPort;
use std::io::Error;

const MIN_BUF_CAPACITY: usize = 4;

pub struct Host<S>
where
    S: SerialPort,
{
    serial_port: S,
    byte_buf: [u8; 1],
}

impl<S> Host<S>
where
    S: SerialPort,
{
    pub const fn new(serial_port: S) -> Self {
        Self {
            serial_port,
            byte_buf: [0],
        }
    }

    pub fn read_packet(&mut self) -> Result<Packet, Error> {
        //let mut buffer = Vec::with_capacity(MIN_BUF_CAPACITY);

        match self.read_byte()? {
            _ => todo!("Implement byte processing"),
        }

        Err(Error::last_os_error())
    }

    fn read_byte(&mut self) -> Result<u8, Error> {
        self.serial_port.read_exact(&mut self.byte_buf)?;
        Ok(self.byte_buf[0])
    }
}
