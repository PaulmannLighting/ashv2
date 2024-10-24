use std::io::{Error, ErrorKind, Read};

use log::{debug, trace, warn};

use crate::packet::Packet;
use crate::protocol::{Stuffing, CANCEL, FLAG, SUBSTITUTE, WAKE, X_OFF, X_ON};
use crate::types::FrameBuffer;
use crate::HexSlice;

pub trait ReadFrame: Read {
    fn read_frame(&mut self, buffer: &mut FrameBuffer) -> std::io::Result<()> {
        buffer.clear();
        let mut error = false;

        for byte in self.bytes() {
            match byte? {
                CANCEL => {
                    trace!("Resetting buffer due to cancel byte.");
                    buffer.clear();
                    error = false;
                }
                FLAG => {
                    trace!("Received flag byte.");

                    if !error && !buffer.is_empty() {
                        debug!("Received frame.");
                        trace!("Buffer: {:#04X}", HexSlice::new(&buffer));
                        buffer.unstuff();
                        trace!("Unstuffed buffer: {:#04X}", HexSlice::new(&buffer));
                        return Ok(());
                    }

                    trace!("Resetting buffer due to error or empty buffer.");
                    trace!("Error condition was: {error}");
                    trace!("Buffer: {:#04X}", HexSlice::new(&buffer));
                    buffer.clear();
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
                    if buffer.is_empty() {
                        debug!("NCP tried to wake us up.");
                    } else if buffer.push(WAKE).is_err() {
                        return Err(Error::new(ErrorKind::OutOfMemory, "Frame buffer overflow."));
                    }
                }
                byte => {
                    if buffer.push(byte).is_err() {
                        return Err(Error::new(ErrorKind::OutOfMemory, "Frame buffer overflow."));
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

impl<T> ReadFrame for T where T: Read {}
