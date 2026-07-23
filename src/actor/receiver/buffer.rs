//! Receive-side frame buffer for `ASHv2` serial input.
//!
//! The buffer consumes chunks from the async serial stream, applies `ASHv2` control-byte
//! handling byte by byte, un-stuffs completed frames, and converts the resulting bytes into
//! typed [`Frame`] values.

use std::io::{ErrorKind, Result};
use std::vec::Drain;

use bytes::Bytes;
use log::{debug, trace, warn};
use tokio::io::AsyncRead;
use tokio_stream::StreamExt;
use tokio_util::io::ReaderStream;

use crate::frame::Frame;
use crate::hex_slice::HexSlice;
use crate::protocol::{ControlByte, Unstuff};
use crate::types::MAX_FRAME_SIZE;

/// Receive-side buffer that reconstructs `ASHv2` frames from serial bytes.
#[derive(Debug)]
pub struct Buffer<T> {
    /// Chunk stream backed by the receiver's serial reader.
    reader: ReaderStream<T>,
    /// Iterator over the currently buffered chunk.
    chunk: <Bytes as IntoIterator>::IntoIter,
    /// Accumulates the current raw frame until a `FLAG` byte terminates it.
    frame: Vec<u8>,
}

impl<T> Buffer<T>
where
    T: AsyncRead,
{
    /// Create a new receive buffer around a serial port.
    #[must_use]
    pub fn new(reader: T) -> Self {
        Self {
            reader: ReaderStream::new(reader),
            chunk: Bytes::new().into_iter(),
            frame: Vec::with_capacity(MAX_FRAME_SIZE),
        }
    }
}

impl<T> Buffer<T>
where
    T: AsyncRead + Unpin,
{
    /// Read the next complete `ASHv2` [`Frame`].
    ///
    /// The method waits until a complete frame is terminated by `FLAG`, then applies byte
    /// unstuffing and frame parsing before returning.
    ///
    /// # Errors
    ///
    /// Returns an error if serial I/O fails, the byte stream ends before another frame is
    /// available, or the completed frame cannot be parsed.
    pub async fn read_frame(&mut self) -> Result<Option<Frame>> {
        self.read_raw_frame().await?.try_into().map(Some)
    }

    async fn read_raw_frame(&mut self) -> Result<Drain<'_, u8>> {
        self.reset_frame();
        let mut error = false;

        while let Some(byte) = self.next_byte().await? {
            match ControlByte::try_from(byte) {
                Ok(control_byte) => match control_byte {
                    ControlByte::Cancel => {
                        trace!("Resetting buffer due to cancel byte.");
                        self.reset_frame();
                        error = false;
                    }
                    ControlByte::Flag => {
                        trace!("Received flag byte.");

                        if !error && !self.frame.is_empty() {
                            debug!("Received frame.");
                            trace!("Buffer: {:#04X}", HexSlice::new(&self.frame));
                            self.frame.unstuff();
                            trace!("Unstuffed buffer: {:#04X}", HexSlice::new(&self.frame));
                            self.warn_if_frame_exceeds_max_frame_size();
                            return Ok(self.frame.drain(..));
                        }

                        trace!("Resetting buffer due to error or empty buffer.");
                        trace!("Error condition was: {error}");
                        trace!("Buffer: {:#04X}", HexSlice::new(&self.frame));
                        self.reset_frame();
                        error = false;
                    }
                    ControlByte::Substitute => {
                        trace!("Received SUBSTITUTE byte. Setting error condition.");
                        error = true;
                    }
                    ControlByte::Xon => {
                        trace!("NCP requested to resume transmission.");
                    }
                    ControlByte::Xoff => {
                        trace!("NCP requested to stop transmission.");
                    }
                    ControlByte::Wake => {
                        if self.frame.is_empty() {
                            debug!("NCP tried to wake us up.");
                        } else {
                            self.frame.push(control_byte.into());
                        }
                    }
                },
                Err(byte) => {
                    self.frame.push(byte);
                }
            }
        }

        trace!("Buffer state: {:#04X}", HexSlice::new(&self.frame));
        self.warn_if_frame_exceeds_max_frame_size();
        Err(ErrorKind::UnexpectedEof.into())
    }

    async fn next_byte(&mut self) -> Result<Option<u8>> {
        loop {
            if let Some(byte) = self.chunk.next() {
                return Ok(Some(byte));
            }

            match self.reader.next().await {
                Some(Ok(bytes)) => {
                    self.chunk = bytes.into_iter();
                }
                Some(Err(error)) => return Err(error),
                None => return Ok(None),
            }
        }
    }

    fn reset_frame(&mut self) {
        self.frame.clear();
        self.frame.shrink_to(MAX_FRAME_SIZE);
    }

    fn warn_if_frame_exceeds_max_frame_size(&self) {
        if self.frame.len() > MAX_FRAME_SIZE {
            warn!("Receiver frame buffer exceeded maximum frame size of {MAX_FRAME_SIZE} bytes.");
        }
    }
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use tokio::runtime::Builder;

    use super::*;

    const FIRST_FRAME_BYTE: u8 = 0x01;
    const SECOND_FRAME_BYTE: u8 = 0x02;
    const EXTRA_FRAME_BYTES: usize = 1;

    #[test]
    fn read_raw_frame_keeps_remaining_chunk_bytes() {
        Builder::new_current_thread()
            .build()
            .expect("runtime should build")
            .block_on(async {
                let flag = u8::from(ControlByte::Flag);
                let input = [FIRST_FRAME_BYTE, flag, SECOND_FRAME_BYTE, flag];
                let mut buffer = Buffer::new(Cursor::new(input));

                let first_frame: Vec<_> = buffer
                    .read_raw_frame()
                    .await
                    .expect("first frame should be readable")
                    .collect();
                let second_frame: Vec<_> = buffer
                    .read_raw_frame()
                    .await
                    .expect("second frame should be readable")
                    .collect();

                assert_eq!(first_frame, [FIRST_FRAME_BYTE]);
                assert_eq!(second_frame, [SECOND_FRAME_BYTE]);
            });
    }

    #[test]
    fn read_raw_frame_shrinks_before_reading_again() {
        Builder::new_current_thread()
            .build()
            .expect("runtime should build")
            .block_on(async {
                let flag = u8::from(ControlByte::Flag);
                let mut input = vec![FIRST_FRAME_BYTE; MAX_FRAME_SIZE + EXTRA_FRAME_BYTES];
                input.push(flag);
                input.push(SECOND_FRAME_BYTE);
                input.push(flag);
                let mut buffer = Buffer::new(Cursor::new(input));

                let oversized_frame: Vec<_> = buffer
                    .read_raw_frame()
                    .await
                    .expect("oversized frame should be readable")
                    .collect();
                let second_frame: Vec<_> = buffer
                    .read_raw_frame()
                    .await
                    .expect("second frame should be readable")
                    .collect();

                assert_eq!(oversized_frame.len(), MAX_FRAME_SIZE + EXTRA_FRAME_BYTES);
                assert_eq!(second_frame, [SECOND_FRAME_BYTE]);
                assert!(buffer.frame.capacity() <= MAX_FRAME_SIZE);
            });
    }
}
