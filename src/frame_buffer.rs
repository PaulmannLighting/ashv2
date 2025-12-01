//! Frame buffer for reading and writing ASH frames.

use core::fmt::{Display, UpperHex};
use std::io::{self, Error, ErrorKind, Read, Write};

use log::{debug, trace};

use crate::frame::Frame;
use crate::protocol::{ControlByte, Stuffing};
use crate::types::RawFrame;
use crate::utils::HexSlice;

/// A buffer for reading and writing ASH frames.
#[derive(Debug)]
pub struct FrameBuffer<T> {
    inner: T,
    buffer: RawFrame,
    xon: bool,
}

impl<T> FrameBuffer<T> {
    /// Create a new `FrameBuffer` with the given inner reader and/or writer.
    #[must_use]
    pub const fn new(inner: T) -> Self {
        Self {
            inner,
            buffer: RawFrame::new(),
            xon: true,
        }
    }

    /// Return the inner reader-writer type.
    #[must_use]
    pub fn into_inner(self) -> T {
        self.inner
    }

    /// Return whether transmission is allowed (XON received).
    #[must_use]
    pub const fn xon(&self) -> bool {
        self.xon
    }

    /// Return a buffer overflow error.
    #[must_use]
    fn buffer_overflow(&self, byte: u8) -> Error {
        trace!("Buffer was: {:#04X}", HexSlice::new(&self.buffer));
        Error::other(format!("Frame buffer overflow: {byte:#04X}"))
    }

    /// Trace the current state of the frame buffer.
    fn trace_buffer(&self) {
        trace!("Frame buffer was: {:#04X}", HexSlice::new(&self.buffer));
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
    pub fn read_frame(&mut self) -> io::Result<Frame> {
        self.read_raw_frame()?.try_into()
    }

    /// Reads an `ASHv2` frame into the buffer.
    ///
    /// # Errors
    ///
    /// Returns an [`Error`] if any I/O or protocol error occurs.
    pub fn read_raw_frame(&mut self) -> io::Result<&[u8]> {
        self.buffer.clear();
        let mut error = false;

        #[expect(clippy::unbuffered_bytes)]
        for byte in (&mut self.inner).bytes() {
            match ControlByte::try_from(byte?) {
                Ok(control_byte) => match control_byte {
                    ControlByte::Cancel => {
                        trace!("Resetting buffer due to cancel byte.");
                        self.buffer.clear();
                        error = false;
                    }
                    ControlByte::Flag => {
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
                    ControlByte::Substitute => {
                        trace!("Received SUBSTITUTE byte. Setting error condition.");
                        error = true;
                    }
                    ControlByte::Xon => {
                        trace!("NCP requested to resume transmission.");
                        self.xon = true;
                    }
                    ControlByte::Xoff => {
                        trace!("NCP requested to stop transmission.");
                        self.xon = false;
                    }
                    ControlByte::Wake => {
                        if self.buffer.is_empty() {
                            debug!("NCP tried to wake us up.");
                        } else if self.buffer.push(control_byte.into()).is_err() {
                            return Err(self.buffer_overflow(control_byte.into()));
                        }
                    }
                },
                Err(byte) => {
                    if self.buffer.push(byte).is_err() {
                        return Err(self.buffer_overflow(byte));
                    }
                }
            }
        }

        self.trace_buffer();
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
    pub fn write_frame<F>(&mut self, frame: F) -> io::Result<()>
    where
        F: IntoIterator<Item = u8> + Display + UpperHex,
    {
        if !self.xon {
            return Err(Error::new(
                ErrorKind::ResourceBusy,
                "Transmission not allowed by NCP (XOFF received).",
            ));
        }

        debug!("Writing frame: {frame}");
        trace!("Frame: {frame:#04X}");
        self.buffer.clear();
        self.buffer.extend(frame);
        trace!("Frame bytes: {:#04X}", HexSlice::new(&self.buffer));
        self.buffer.stuff()?;
        trace!("Stuffed bytes: {:#04X}", HexSlice::new(&self.buffer));
        self.buffer
            .push(ControlByte::Flag.into())
            .map_err(|_| self.buffer_overflow(ControlByte::Flag.into()))?;
        trace!("Writing bytes: {:#04X}", HexSlice::new(&self.buffer));
        self.inner.write_all(&self.buffer)?;
        self.inner.flush()
    }
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use super::*;
    use crate::frame::Data;

    #[test]
    fn test_read_frame() {
        let data = vec![
            ControlByte::Flag as u8,
            0x7D,
            0x5E,
            0x7D,
            0x31,
            0x7D,
            0x33,
            0x7D,
            0x38,
            0x7D,
            0x3A,
            0x7D,
            0x5D,
            ControlByte::Flag as u8,
            0x7D,
            0x5E,
            0x7D,
            0x31,
            0x7D,
            0x33,
            0x7D,
            0x38,
            0x7D,
            0x3A,
            0x7D,
            0x5D,
            ControlByte::Flag as u8,
        ];
        let mut buffer = FrameBuffer::new(Cursor::new(data));
        let reference = Data::try_from([0x7Eu8, 0x11, 0x13, 0x18, 0x1A, 0x7D].as_slice())
            .expect("Reference data should be valid.");

        let Frame::Data(data) = buffer.read_frame().expect("A data frame should be read.") else {
            panic!("Expected a Data frame");
        };

        assert_eq!(data.ack_num(), reference.ack_num());
        assert_eq!(data.frame_num(), reference.frame_num());
        assert_eq!(data.is_retransmission(), reference.is_retransmission());
        assert_eq!(data.into_payload(), reference.clone().into_payload());

        let Frame::Data(data) = buffer.read_frame().expect("A data frame should be read.") else {
            panic!("Expected a Data frame");
        };

        assert_eq!(data.ack_num(), reference.ack_num());
        assert_eq!(data.frame_num(), reference.frame_num());
        assert_eq!(data.is_retransmission(), reference.is_retransmission());
        assert_eq!(data.into_payload(), reference.into_payload());
    }
}
