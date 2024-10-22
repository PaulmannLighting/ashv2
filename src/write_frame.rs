use std::fmt::{LowerHex, UpperHex};
use std::io::{Error, ErrorKind, Write};

use log::{debug, trace};

use crate::frame::Frame;
use crate::protocol::{Stuffing, FLAG};
use crate::types::FrameBuffer;
use crate::HexSlice;

pub trait WriteFrame: Write {
    /// Writes an ASH [`Frame`].
    ///
    /// # Errors
    ///
    /// Returns an [Error] if the serial port write operation failed.
    fn write_frame<F>(&mut self, frame: &F, buffer: &mut FrameBuffer) -> std::io::Result<()>
    where
        F: Frame + LowerHex + UpperHex,
    {
        debug!("Writing frame: {frame}");
        trace!("Frame: {frame:#04X}");
        buffer.clear();
        frame.buffer(buffer).map_err(|()| {
            Error::new(
                ErrorKind::OutOfMemory,
                "Could not append frame bytes to buffer.",
            )
        })?;
        trace!("Frame bytes: {:#04X}", HexSlice::new(buffer));
        buffer.stuff()?;
        trace!("Stuffed bytes: {:#04X}", HexSlice::new(buffer));
        buffer
            .push(FLAG)
            .map_err(|_| Error::new(ErrorKind::OutOfMemory, "Could not append flag byte."))?;
        trace!("Writing bytes: {:#04X}", HexSlice::new(buffer));
        self.write_all(&buffer)?;
        self.flush()
    }
}

impl<T> WriteFrame for T where T: Write {}
