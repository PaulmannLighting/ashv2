//! Frame buffer for reading and writing ASH frames.

use std::io::{self, BufReader, Error, ErrorKind, Read};

use log::{debug, trace};

use crate::frame::Frame;
use crate::protocol::{ControlByte, Unstuff};
use crate::utils::HexSlice;

/// A buffer for reading and writing ASH frames.
#[derive(Debug)]
pub struct Buffer<T> {
    reader: BufReader<T>,
    buffer: Vec<u8>,
}

impl<T> Buffer<T> {
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
impl<T> Buffer<T>
where
    T: Read,
{
    /// Create a new `FrameBuffer` with the given inner reader and/or writer.
    #[must_use]
    pub fn new(serial_port: T) -> Self {
        Self {
            reader: BufReader::new(serial_port),
            buffer: Vec::new(),
        }
    }

    /// Read an `ASHv2` [`Frame`].
    ///
    /// # Errors
    ///
    /// Returns an [`Error`] if any I/O, protocol or parsing error occurs.
    pub fn read_frame(&mut self) -> io::Result<Frame> {
        self.buffer.clear();
        let mut error = false;

        for byte in (&mut self.reader).bytes() {
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
                            return self.buffer.drain(..).as_slice().try_into();
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
                    }
                    ControlByte::Xoff => {
                        trace!("NCP requested to stop transmission.");
                    }
                    ControlByte::Wake => {
                        if self.buffer.is_empty() {
                            debug!("NCP tried to wake us up.");
                        } else {
                            self.buffer.push(control_byte.into())
                        }
                    }
                },
                Err(byte) => {
                    self.buffer.push(byte);
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
        let mut buffer = Buffer::new(Cursor::new(data));
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
