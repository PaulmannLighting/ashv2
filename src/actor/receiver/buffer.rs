//! Receive-side frame buffer for `ASHv2` serial input.
//!
//! The buffer consumes one byte at a time from the async serial stream, applies `ASHv2`
//! control-byte handling, un-stuffs completed frames, and converts the resulting bytes into
//! typed [`Frame`] values.

use std::io::{ErrorKind, Result};
use std::vec::Drain;

use log::{debug, trace};
use tokio::io::AsyncRead;

use super::AsyncBufStream;
use crate::frame::Frame;
use crate::hex_slice::HexSlice;
use crate::protocol::{ControlByte, Unstuff};
use crate::types::MAX_FRAME_SIZE;

/// Receive-side buffer that reconstructs `ASHv2` frames from serial bytes.
#[derive(Debug)]
pub struct Buffer<T> {
    /// Byte stream backed by the receiver's serial port clone.
    serial_port: AsyncBufStream<T>,
    /// Accumulates the current raw frame until a `FLAG` byte terminates it.
    frame: Vec<u8>,
}

impl<T> Buffer<T>
where
    T: AsyncRead,
{
    /// Create a new receive buffer around a serial port.
    #[must_use]
    pub fn new(serial_port: T) -> Self {
        Self {
            serial_port: AsyncBufStream::new(serial_port),
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
