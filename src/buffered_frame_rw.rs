use std::cell::LazyCell;
use std::fmt::{LowerHex, UpperHex};
use std::io::{Error, ErrorKind, Read, Write};

use log::{debug, trace, warn};

use crate::frame::Frame;
use crate::packet::Packet;
use crate::protocol::{Stuffing, CANCEL, FLAG, SUBSTITUTE, WAKE, X_OFF, X_ON};
use crate::types::MAX_FRAME_SIZE;
use crate::HexSlice;

const BUFFER_OVERFLOW: LazyCell<Error> =
    LazyCell::new(|| Error::new(ErrorKind::OutOfMemory, "Frame buffer overflow."));

#[derive(Debug)]
pub struct FrameBuffer<T> {
    inner: T,
    buffer: heapless::Vec<u8, MAX_FRAME_SIZE>,
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
                        return Err(*BUFFER_OVERFLOW);
                    }
                }
                byte => {
                    if self.buffer.push(byte).is_err() {
                        return Err(*BUFFER_OVERFLOW);
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
            .map_err(|()| *BUFFER_OVERFLOW)?;
        trace!("Frame bytes: {:#04X}", HexSlice::new(&self.buffer));
        self.buffer.stuff()?;
        trace!("Stuffed bytes: {:#04X}", HexSlice::new(&self.buffer));
        self.buffer.push(FLAG).map_err(|_| *BUFFER_OVERFLOW)?;
        trace!("Writing bytes: {:#04X}", HexSlice::new(&self.buffer));
        self.inner.write_all(&self.buffer)?;
        self.inner.flush()
    }
}
