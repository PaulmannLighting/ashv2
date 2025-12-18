//! Frame buffer for reading and writing ASH frames.

use core::fmt::{Display, UpperHex};
use std::io::{self, Error};

use log::{debug, trace};
use serialport::SerialPort;

use crate::protocol::{ControlByte, Stuff};
use crate::types::RawFrame;
use crate::utils::HexSlice;

/// A buffer for reading and writing ASH frames.
#[derive(Debug)]
pub struct Buffer<T> {
    inner: T,
    frame: RawFrame,
}

impl<T> Buffer<T> {
    /// Create a new `FrameBuffer` with the given inner reader and/or writer.
    #[must_use]
    pub const fn new(inner: T) -> Self {
        Self {
            inner,
            frame: RawFrame::new(),
        }
    }
}

impl<T> Buffer<T>
where
    T: SerialPort,
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
        debug!("Writing frame: {frame}");
        trace!("Frame: {frame:#04X}");
        self.frame.clear();
        self.frame.extend(frame);
        trace!("Frame bytes: {:#04X}", HexSlice::new(&self.frame));
        self.frame.stuff()?;
        trace!("Stuffed bytes: {:#04X}", HexSlice::new(&self.frame));
        self.frame
            .push(ControlByte::Flag.into())
            .map_err(|byte| Error::other(format!("Frame buffer overflow: {byte:#04X}")))?;
        trace!("Writing bytes: {:#04X}", HexSlice::new(&self.frame));
        self.inner.write_all(&self.frame)?;
        self.inner.flush()
    }
}
