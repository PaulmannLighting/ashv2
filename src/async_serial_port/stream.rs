use std::io::Result;

use bytes::BytesMut;

use super::Reader;

pub struct Stream {
    reader: Reader,
    buffer: <BytesMut as IntoIterator>::IntoIter,
}

impl Stream {
    #[must_use]
    pub fn new(reader: Reader) -> Self {
        Self {
            reader,
            buffer: BytesMut::new().into_iter(),
        }
    }

    pub async fn next(&mut self) -> Result<u8> {
        loop {
            if let Some(byte) = self.buffer.next() {
                return Ok(byte);
            }

            self.buffer = self.reader.read().await?.into_iter();
        }
    }
}
