//! Frame buffer for reading and writing ASH frames.

use std::io::{self, BufReader, Error, ErrorKind, Read};
use std::time::Duration;
use std::vec::Drain;

use log::{debug, trace};
use serialport::SerialPort;
use tokio::time::sleep;

use crate::frame::Frame;
use crate::protocol::{ControlByte, Unstuff};
use crate::utils::HexSlice;

/// A buffer for reading and writing ASH frames.
#[derive(Debug)]
pub struct Buffer<T> {
    reader: BufReader<T>,
    frame: Vec<u8>,
    timeout: Option<Duration>,
}

/// The `FrameBuffer` can read `ASHv2` frames if `T` implements [`Read`].
impl<T> Buffer<T>
where
    T: Read,
{
    /// Create a new `FrameBuffer` with the given inner reader and/or writer.
    #[must_use]
    pub fn new(serial_port: T, timeout: Option<Duration>) -> Self {
        Self {
            reader: BufReader::new(serial_port),
            frame: Vec::new(),
            timeout,
        }
    }

    /// Read an `ASHv2` [`Frame`].
    ///
    /// # Errors
    ///
    /// Returns an [`Error`] if any I/O, protocol or parsing error occurs.
    pub async fn read_frame(&mut self) -> io::Result<Option<Frame>> {
        let error = match self.read_raw_frame() {
            Ok(raw_frame) => return raw_frame.as_slice().try_into().map(Some),
            Err(error) => error,
        };

        if error.kind() == ErrorKind::TimedOut {
            if let Some(timeout) = self.timeout {
                sleep(timeout).await;
            }
            Ok(None)
        } else {
            Err(error)
        }
    }

    fn read_raw_frame(&mut self) -> io::Result<Drain<'_, u8>> {
        self.frame.clear();
        let mut error = false;

        for byte in (&mut self.reader).bytes() {
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
        Err(Error::new(
            ErrorKind::UnexpectedEof,
            "Byte stream terminated unexpectedly.",
        ))
    }
}

impl<T> From<T> for Buffer<T>
where
    T: SerialPort,
{
    fn from(serial_port: T) -> Self {
        let timeout = serial_port.timeout();
        Self::new(serial_port, Some(timeout))
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
        let mut buffer = Buffer::new(Cursor::new(data), None);
        let reference = Data::try_from([0x7Eu8, 0x11, 0x13, 0x18, 0x1A, 0x7D].as_slice())
            .expect("Reference data should be valid.");

        let Frame::Data(data) = buffer
            .read_raw_frame()
            .expect("A data frame should be read.")
            .as_slice()
            .try_into()
            .expect("A data frame should be read.")
        else {
            panic!("Expected a Data frame");
        };

        assert_eq!(data.ack_num(), reference.ack_num());
        assert_eq!(data.frame_num(), reference.frame_num());
        assert_eq!(data.is_retransmission(), reference.is_retransmission());
        assert_eq!(data.into_payload(), reference.clone().into_payload());

        let Frame::Data(data) = buffer
            .read_raw_frame()
            .expect("A data frame should be read.")
            .as_slice()
            .try_into()
            .expect("A data frame should be read.")
        else {
            panic!("Expected a Data frame");
        };

        assert_eq!(data.ack_num(), reference.ack_num());
        assert_eq!(data.frame_num(), reference.frame_num());
        assert_eq!(data.is_retransmission(), reference.is_retransmission());
        assert_eq!(data.into_payload(), reference.into_payload());
    }
}
