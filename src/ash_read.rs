use crate::packet::Packet;
use crate::protocol::{Unstuff, CANCEL, FLAG, SUBSTITUTE, WAKE, X_OFF, X_ON};
use crate::Error;
use log::{debug, trace};
use std::io::{ErrorKind, Read};

pub trait AshRead: Read {
    /// Read an ASH frame [`Packet`].
    ///
    /// # Arguments
    /// * `buffer` The buffer used for input buffering.
    ///
    /// # Errors
    /// Returns an [`Error`] if any I/O, protocol or parsing error occur.
    fn read_frame(&mut self, buffer: &mut Vec<u8>) -> Result<Packet, Error> {
        self.read_frame_raw(buffer)?;
        Ok(Packet::try_from(buffer.as_slice())?)
    }

    /// Reads a raw ASH frame as [`Vec<[u8]>`].
    ///
    /// # Arguments
    /// * `buffer` The buffer used for input buffering.
    ///
    /// # Errors
    /// Returns an [`Error`] if any I/O, protocol or parsing error occur.
    fn read_frame_raw(&mut self, buffer: &mut Vec<u8>) -> Result<(), Error> {
        buffer.clear();
        let mut byte = [0; 1];
        let mut error = false;

        loop {
            if let Err(error) = self.read_exact(&mut byte) {
                if error.kind() == ErrorKind::UnexpectedEof {
                    continue;
                }

                return Err(error.into());
            }

            match byte[0] {
                CANCEL => {
                    debug!("Resetting buffer due to cancel byte.");
                    trace!("Error condition: {error}");
                    trace!("Buffer content: {:#04X?}", buffer);
                    buffer.clear();
                    error = false;
                }
                FLAG => {
                    if !error && !buffer.is_empty() {
                        debug!("Received frame.");
                        trace!("Frame details: {:#04X?}", buffer);
                        buffer.unstuff();
                        return Ok(());
                    }

                    debug!("Resetting buffer due to error or empty buffer.");
                    trace!("Error condition: {error}");
                    trace!("Buffer content: {:#04X?}", buffer);
                    buffer.clear();
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
                byte => buffer.push(byte),
            }
        }
    }
}

impl<T> AshRead for T where T: Read {}
