use std::fmt::{LowerHex, UpperHex};
use std::io::{Error, ErrorKind, Read, Write};

use log::{debug, trace, warn};

use crate::frame::Frame;
use crate::packet::Packet;
use crate::protocol::{Stuffing, CANCEL, FLAG, SUBSTITUTE, WAKE, X_OFF, X_ON};
use crate::types::FrameVec;
use crate::HexSlice;

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

impl<T> FrameBuffer<T>
where
    T: Read,
{
    /// Read an ASH [`Packet`].
    ///
    /// # Errors
    ///
    /// Returns an [`Error`] if any I/O, protocol or parsing error occurs.
    pub fn read_packet(&mut self) -> std::io::Result<Packet> {
        Packet::try_from(self.read_frame()?)
    }

    /// Reads an ASH frame into the buffer.
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

impl<T> FrameBuffer<T>
where
    T: Write,
{
    /// Writes an ASH [`Frame`].
    ///
    /// # Errors
    ///
    /// Returns an [Error] if the serial port write operation failed.
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
