use std::fmt::{LowerHex, UpperHex};
use std::io::{Error, ErrorKind, Read, Write};

use log::{debug, trace, warn};

use crate::frame::Frame;
use crate::packet::Packet;
use crate::protocol::{Stuffing, CANCEL, FLAG, SUBSTITUTE, WAKE, X_OFF, X_ON};
use crate::types::FrameVec;
use crate::HexSlice;

/// A buffer for reading and writing ASH frames.
#[derive(Debug)]
pub struct FrameBuffer<T> {
    inner: T,
    buffer: FrameVec,
}

impl<T> FrameBuffer<T> {
    #[must_use]
    pub const fn new(inner: T) -> Self {
        Self {
            inner,
            buffer: FrameVec::new(),
        }
    }

    #[must_use]
    fn buffer_overflow() -> Error {
        Error::new(ErrorKind::Other, "Frame buffer overflow.")
    }
}

/// The `FrameBuffer` can read  `ASHv2` frames if `T` implements [`Read`].
impl<T> FrameBuffer<T>
where
    T: Read,
{
    /// Read an `ASHv2` [`Packet`].
    ///
    /// # Errors
    ///
    /// Returns an [`Error`] if any I/O, protocol or parsing error occurs.
    pub fn read_packet(&mut self) -> std::io::Result<Packet> {
        self.read_frame()?.try_into()
    }

    /// Reads an `ASHv2` frame into the buffer.
    ///
    /// # Errors
    /// Returns an [`Error`] if any I/O or protocol error occurs.
    pub fn read_frame(&mut self) -> std::io::Result<&[u8]> {
        self.buffer.clear();
        let mut error = false;

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
                        return Err(Self::buffer_overflow());
                    }
                }
                byte => {
                    if self.buffer.push(byte).is_err() {
                        return Err(Self::buffer_overflow());
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
    /// Writes an `ASHv2` [`Frame`].
    ///
    /// # Errors
    ///
    /// Returns an [Error] if the write operation failed or a buffer overflow occurred.
    pub fn write_frame<F>(&mut self, frame: &F) -> std::io::Result<()>
    where
        F: Frame + LowerHex + UpperHex,
    {
        debug!("Writing frame: {frame}");
        trace!("Frame: {frame:#04X}");
        self.buffer.clear();
        frame
            .buffer(&mut self.buffer)
            .map_err(|()| Self::buffer_overflow())?;
        trace!("Frame bytes: {:#04X}", HexSlice::new(&self.buffer));
        self.buffer.stuff()?;
        trace!("Stuffed bytes: {:#04X}", HexSlice::new(&self.buffer));
        self.buffer
            .push(FLAG)
            .map_err(|_| Self::buffer_overflow())?;
        trace!("Writing bytes: {:#04X}", HexSlice::new(&self.buffer));
        self.inner.write_all(&self.buffer)?;
        self.inner.flush()
    }
}
