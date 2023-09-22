use crate::packet::Packet;
use std::io::{Error, Read, Write};

const MIN_BUF_CAPACITY: usize = 4;

pub struct Host<R, W>
where
    R: Read,
    W: Write,
{
    reader: R,
    writer: W,
}

impl<R, W> Host<R, W>
where
    R: Read,
    W: Write,
{
    pub fn new(reader: R, writer: W) -> Self {
        Self { reader, writer }
    }

    pub fn read_packet(&mut self) -> Result<Packet, Error> {
        //let mut buffer = Vec::with_capacity(MIN_BUF_CAPACITY);

        let mut header: [u8; 1] = [0];
        self.reader.read_exact(&mut header)?;

        Err(Error::last_os_error())
    }
}
