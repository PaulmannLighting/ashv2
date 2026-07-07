//! Frame buffer for reading and writing ASH frames.

use std::io::{ErrorKind, Result};
use std::vec::Drain;

use log::{debug, trace};
use serialport::SerialPort;

use crate::async_buf_stream::AsyncBufStream;
use crate::async_serial_port::AsyncSerialPort;
use crate::frame::Frame;
use crate::hex_slice::HexSlice;
use crate::protocol::{ControlByte, Unstuff};
use crate::types::MAX_FRAME_SIZE;

/// A buffer for reading and writing ASH frames.
#[derive(Debug)]
pub struct Buffer<T> {
    serial_port: AsyncBufStream<AsyncSerialPort<T>>,
    frame: Vec<u8>,
}

/// The `FrameBuffer` can read `ASHv2` frames if `T` implements [`Read`].
impl<T> Buffer<T>
where
    T: SerialPort,
{
    /// Create a new `FrameBuffer` with the given inner reader and/or writer.
    #[must_use]
    pub fn new(serial_port: T) -> Self {
        Self {
            serial_port: AsyncBufStream::new(AsyncSerialPort(serial_port)),
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
    pub async fn read_frame(&mut self) -> Result<Option<Frame>> {
        self.read_raw_frame().await?.try_into().map(Some)
    }

    async fn read_raw_frame(&mut self) -> Result<Drain<'_, u8>> {
        self.frame.clear();
        let mut error = false;

        while let Some(byte) = self.serial_port.next().await {
            match ControlByte::try_from(byte?) {
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

        trace!("Buffer state: {:#04X}", HexSlice::new(&self.frame));
        Err(ErrorKind::UnexpectedEof.into())
    }
}
