//! Frame buffer for reading and writing ASH frames.

use core::fmt::{Display, UpperHex};
use std::io::{Error, ErrorKind, Read, Write};

use log::{debug, trace, warn};

use crate::frame::Frame;
use crate::protocol::{CANCEL, FLAG, SUBSTITUTE, Stuffing, WAKE, X_OFF, X_ON};
use crate::to_buffer::ToBuffer;
use crate::types::RawFrame;
use crate::utils::HexSlice;
use crate::validate::Validate;

/// A buffer for reading and writing ASH frames.
#[derive(Debug)]
pub struct FrameBuffer<T> {
    inner: T,
    buffer: RawFrame,
}

impl<T> FrameBuffer<T> {
    /// Create a new `FrameBuffer` with the given inner reader and/or writer.
    #[must_use]
    pub const fn new(inner: T) -> Self {
        Self {
            inner,
            buffer: RawFrame::new(),
        }
    }

    /// Return the inner reader-writer type.
    #[must_use]
    pub fn into_inner(self) -> T {
        self.inner
    }

    #[must_use]
    fn buffer_overflow(byte: u8) -> Error {
        Error::other(format!("Frame buffer overflow: {byte:#04X}"))
    }
}

/// The `FrameBuffer` can read `ASHv2` frames if `T` implements [`Read`].
impl<T> FrameBuffer<T>
where
    T: Read,
{
    /// Read an `ASHv2` [`Frame`].
    ///
    /// # Errors
    ///
    /// Returns an [`Error`] if any I/O, protocol or parsing error occurs.
    pub fn read_frame(&mut self) -> std::io::Result<Frame> {
        let frame: Frame = self.read_raw_frame()?.try_into()?;

        if frame.is_crc_valid() {
            Ok(frame)
        } else {
            Err(Error::new(
                ErrorKind::InvalidData,
                "Frame is not CRC valid.",
            ))
        }
    }

    /// Reads an `ASHv2` frame into the buffer.
    ///
    /// # Errors
    ///
    /// Returns an [`Error`] if any I/O or protocol error occurs.
    pub fn read_raw_frame(&mut self) -> std::io::Result<&[u8]> {
        self.buffer.clear();
        let mut error = false;

        #[allow(clippy::unbuffered_bytes)]
        for byte in (&mut self.inner).bytes() {
            match byte? {
                CANCEL => {
                    trace!("Resetting buffer due to cancel byte.");
                    self.buffer.clear();
                    error = false;
                }
                FLAG => {
                    trace!("Received flag byte.");

                    if !error && !self.buffer.is_empty() {
                        debug!("Received frame.");
                        trace!("Buffer: {:#04X}", HexSlice::new(&self.buffer));
                        self.buffer.unstuff();
                        trace!("Unstuffed buffer: {:#04X}", HexSlice::new(&self.buffer));
                        return Ok(&self.buffer);
                    }

                    trace!("Resetting buffer due to error or empty buffer.");
                    trace!("Error condition was: {error}");
                    trace!("Buffer: {:#04X}", HexSlice::new(&self.buffer));
                    self.buffer.clear();
                    error = false;
                }
                SUBSTITUTE => {
                    trace!("Received SUBSTITUTE byte. Setting error condition.");
                    error = true;
                }
                X_ON => {
                    warn!("NCP requested to resume transmission. Ignoring.");
                }
                X_OFF => {
                    warn!("NCP requested to stop transmission. Ignoring.");
                }
                WAKE => {
                    if self.buffer.is_empty() {
                        debug!("NCP tried to wake us up.");
                    } else if self.buffer.push(WAKE).is_err() {
                        return Err(Self::buffer_overflow(WAKE));
                    }
                }
                byte => {
                    if self.buffer.push(byte).is_err() {
                        return Err(Self::buffer_overflow(byte));
                    }
                }
            }
        }

        Err(Error::new(
            ErrorKind::UnexpectedEof,
            "Byte stream terminated unexpectedly.",
        ))
    }
}

/// The `FrameBuffer` can write `ASHv2` frames if `T` implements [`Write`].
impl<T> FrameBuffer<T>
where
    T: Write,
{
    /// Write an `ASHv2` frame into the buffer.
    ///
    /// # Errors
    ///
    /// Returns an [Error] if the write operation failed or a buffer overflow occurred.
    pub fn write_frame<F>(&mut self, frame: &F) -> std::io::Result<()>
    where
        F: ToBuffer + Display + UpperHex,
    {
        debug!("Writing frame: {frame}");
        trace!("Frame: {frame:#04X}");
        self.buffer.clear();
        frame.buffer(&mut self.buffer)?;
        trace!("Frame bytes: {:#04X}", HexSlice::new(&self.buffer));
        self.buffer.stuff()?;
        trace!("Stuffed bytes: {:#04X}", HexSlice::new(&self.buffer));
        self.buffer
            .push(FLAG)
            .map_err(|_| Self::buffer_overflow(FLAG))?;
        trace!("Writing bytes: {:#04X}", HexSlice::new(&self.buffer));
        self.inner.write_all(&self.buffer)?;
        self.inner.flush()
    }
}
