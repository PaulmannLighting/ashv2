use crate::buffer::FrameBuffer;
use crate::packet::Packet;
use crate::protocol::{Unstuff, CANCEL, FLAG, SUBSTITUTE, WAKE, X_OFF, X_ON};
use crate::Error;
use log::{debug, trace};
use std::io::{ErrorKind, Read, Seek, Write};
use std::ops::Deref;

pub trait AshRead: Read {
    /// Read an ASH frame [`Packet`].
    ///
    /// # Arguments
    /// * `buffer` The buffer used for input buffering.
    ///
    /// # Errors
    /// Returns an [`Error`] if any I/O, protocol or parsing error occur.
    fn read_frame(&mut self, buffer: &mut FrameBuffer) -> Result<Packet, Error> {
        self.read_frame_raw(buffer)?;
        Ok(Packet::try_from((*buffer).deref())?)
    }

    /// Reads a raw ASH frame as [`Vec<[u8]>`].
    ///
    /// # Arguments
    /// * `buffer` The buffer used for input buffering.
    ///
    /// # Errors
    /// Returns an [`Error`] if any I/O, protocol or parsing error occur.
    fn read_frame_raw(&mut self, buffer: &mut FrameBuffer) -> Result<(), Error> {
        buffer.rewind()?;
        let mut error = false;

        for byte in self.bytes() {
            match byte? {
                CANCEL => {
                    debug!("Resetting buffer due to cancel byte.");
                    trace!("Error condition: {error}");
                    trace!("{:#04X?}", &*buffer);
                    buffer.rewind()?;
                    error = false;
                }
                FLAG => {
                    debug!("Received flag byte.");

                    if !error && !buffer.is_empty() {
                        debug!("Frame complete.");
                        trace!("{:#04X?}", &*buffer);
                        buffer.unstuff();
                        return Ok(());
                    }

                    debug!("Resetting buffer due to error or empty buffer.");
                    trace!("Error condition: {error}");
                    trace!("{:#04X?}", &*buffer);
                    buffer.rewind()?;
                    error = false;
                }
                SUBSTITUTE => {
                    debug!("Received SUBSTITUTE byte. Setting error condition.");
                    error = true;
                }
                X_ON => {
                    debug!("NCP requested to stop transmission.");
                }
                X_OFF => {
                    debug!("NCP requested to resume transmission.");
                }
                WAKE => {
                    debug!("NCP tried to wake us up.");
                }
                byte => buffer.write_all(&[byte])?,
            }
        }

        Err(std::io::Error::new(
            ErrorKind::UnexpectedEof,
            "byte stream terminated unexpectedly",
        )
        .into())
    }
}

impl<T> AshRead for T where T: Read {}
