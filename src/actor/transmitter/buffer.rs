//! Frame buffer for reading and writing ASH frames.

use core::fmt::{Display, UpperHex};
use std::io::{self, Error, ErrorKind};

use log::{debug, trace, warn};
use serialport::SerialPort;

use crate::protocol::{ControlByte, Stuff};
use crate::types::RawFrame;
use crate::utils::HexSlice;

/// A buffer for reading and writing ASH frames.
#[derive(Debug)]
pub struct Buffer<T> {
    inner: T,
    buffer: RawFrame,
}

impl<T> Buffer<T> {
    /// Create a new `FrameBuffer` with the given inner reader and/or writer.
    #[must_use]
    pub const fn new(inner: T) -> Self {
        Self {
            inner,
            buffer: RawFrame::new(),
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
        if !self
            .inner
            .read_clear_to_send()
            .inspect_err(|error| {
                warn!("Failed to read CTS line: {error}");
            })
            .unwrap_or(true)
        {
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
            .map_err(|byte| Error::other(format!("Frame buffer overflow: {byte:#04X}")))?;
        trace!("Writing bytes: {:#04X}", HexSlice::new(&self.buffer));
        self.inner.write_all(&self.buffer)?;
        self.inner.flush()
    }
}
