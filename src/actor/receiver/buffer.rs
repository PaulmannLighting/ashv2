//! Frame buffer for reading and writing ASH frames.

use std::io::Result;
use std::vec::Drain;

use bytes::BytesMut;
use log::{debug, trace};
use serialport::SerialPort;

use crate::frame::Frame;
use crate::hex_slice::HexSlice;
use crate::protocol::{ControlByte, Unstuff};
use crate::types::MAX_FRAME_SIZE;

/// A buffer for reading and writing ASH frames.
#[derive(Debug)]
pub struct Buffer<T> {
    serial_port: T,
    bytes: <BytesMut as IntoIterator>::IntoIter,
    frame: Vec<u8>,
}

/// The `FrameBuffer` can read `ASHv2` frames if `T` implements [`Read`].
impl<T> Buffer<T> {
    /// Create a new `FrameBuffer` with the given inner reader and/or writer.
    #[must_use]
    pub fn new(serial_port: T) -> Self {
        Self {
            serial_port,
            bytes: BytesMut::new().into_iter(),
            frame: Vec::with_capacity(MAX_FRAME_SIZE),
        }
    }
}

impl<T> Buffer<T>
where
    T: SerialPort,
{
    /// Read an `ASHv2` [`Frame`].
    ///
    /// # Errors
    ///
    /// Returns an [`Error`] if any I/O, protocol or parsing error occurs.
    pub fn read_frame(&mut self) -> Result<Option<Frame>> {
        self.read_raw_frame()?.try_into().map(Some)
    }

    fn read_raw_frame(&mut self) -> Result<Drain<'_, u8>> {
        self.frame.clear();
        let mut error = false;

        loop {
            let Some(byte) = self.bytes.next() else {
                let len = self.serial_port.bytes_to_read()?;

                if len == 0 {
                    continue;
                }

                let mut bytes = BytesMut::zeroed(len as usize);
                self.serial_port.read_exact(&mut bytes)?;
                self.bytes = bytes.into_iter();
                continue;
            };

            match ControlByte::try_from(byte) {
                Ok(control_byte) => match control_byte {
                    ControlByte::Cancel => {
                        trace!("Resetting buffer due to cancel byte.");
                        self.frame.clear();
                        error = false;
                    }
                    ControlByte::Flag => {
                        trace!("Received flag byte.");

                        if !error && !self.frame.is_empty() {
                            debug!("Received frame.");
                            trace!("Buffer: {:#04X}", HexSlice::new(&self.frame));
                            self.frame.unstuff();
                            trace!("Unstuffed buffer: {:#04X}", HexSlice::new(&self.frame));
                            return Ok(self.frame.drain(..));
                        }

                        trace!("Resetting buffer due to error or empty buffer.");
                        trace!("Error condition was: {error}");
                        trace!("Buffer: {:#04X}", HexSlice::new(&self.frame));
                        self.frame.clear();
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
    }
}
