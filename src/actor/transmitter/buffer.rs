//! Transmit-side frame buffer for `ASHv2` serial output.

use core::fmt::{Display, UpperHex};
use std::io::{self, Error};

use log::{debug, trace};
use tokio::io::{AsyncWrite, AsyncWriteExt};

use crate::hex_slice::HexSlice;
use crate::protocol::{ControlByte, Stuff};
use crate::types::RawFrame;

/// Transmit-side buffer that encodes `ASHv2` frames for serial writes.
#[derive(Debug)]
pub struct Buffer<T> {
    /// Async writer for the serial transmit path.
    inner: T,
    /// Reusable frame buffer used for stuffing and termination.
    frame: RawFrame,
}

impl<T> Buffer<T> {
    /// Create a new transmit buffer around an async writer.
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
    T: AsyncWrite + Unpin,
{
    /// Write an `ASHv2` frame into the buffer.
    ///
    /// # Errors
    ///
    /// Returns an error if the write operation failed or the frame buffer overflowed.
    pub async fn write_frame<F>(&mut self, frame: F) -> io::Result<()>
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
        self.inner.write_all(&self.frame).await?;
        self.inner.flush().await
    }
}
