use std::io::{ErrorKind, Read};

use log::{debug, trace};

use crate::error::Error;
use crate::frame_buffer::FrameBuffer;
use crate::packet::Packet;
use crate::protocol::{Unstuff, CANCEL, FLAG, SUBSTITUTE, WAKE, X_OFF, X_ON};

pub trait AshRead: Read {
    /// Read an ASH [`Packet`].
    ///
    /// # Arguments
    /// * `buffer` The buffer used for input buffering.
    ///
    /// # Errors
    /// Returns an [`Error`] if any I/O, protocol or parsing error occurs.
    fn read_packet_buffered(&mut self, buffer: &mut FrameBuffer) -> Result<Packet, Error> {
        self.read_frame_buffered(buffer)?;
        Ok(Packet::try_from(&**buffer)?)
    }

    /// Reads an ASH frame into a [`FrameBuffer`].
    ///
    /// # Arguments
    /// * `buffer` The buffer used for input buffering.
    ///
    /// # Errors
    /// Returns an [`std::io::Error`] if any I/O or protocol error occurs.
    fn read_frame_buffered(&mut self, buffer: &mut FrameBuffer) -> std::io::Result<()> {
        buffer.clear();
        let mut error = false;

        for byte in self.bytes() {
            match byte? {
                CANCEL => {
                    debug!("Resetting buffer due to cancel byte.");
                    trace!("Error condition: {error}");
                    trace!("Buffer: {:#04X?}", buffer);
                    buffer.clear();
                    error = false;
                }
                FLAG => {
                    debug!("Received flag byte.");

                    if !error && !buffer.is_empty() {
                        debug!("Frame complete.");
                        trace!("Buffer: {:#04X?}", buffer);
                        buffer.unstuff();
                        trace!("Unstuffed buffer: {:#04X?}", buffer);
                        return Ok(());
                    }

                    debug!("Resetting buffer due to error or empty buffer.");
                    trace!("Error condition: {error}");
                    trace!("Buffer: {:#04X?}", buffer);
                    buffer.clear();
                    error = false;
                }
                SUBSTITUTE => {
                    debug!("Received SUBSTITUTE byte. Setting error condition.");
                    error = true;
                }
                X_ON => {
                    debug!("NCP requested to resume transmission.");
                }
                X_OFF => {
                    debug!("NCP requested to stop transmission.");
                }
                WAKE => {
                    debug!("NCP tried to wake us up.");
                }
                byte => {
                    if buffer.push(byte).is_err() {
                        return Err(std::io::Error::new(
                            ErrorKind::OutOfMemory,
                            "Buffer overflow.",
                        ));
                    }
                }
            }
        }

        Err(std::io::Error::new(
            ErrorKind::UnexpectedEof,
            "byte stream terminated unexpectedly",
        ))
    }
}

impl<T> AshRead for T where T: Read {}
