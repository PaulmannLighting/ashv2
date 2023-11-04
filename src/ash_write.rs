use crate::buffer::FrameBuffer;
use crate::frame::Frame;
use crate::protocol::{Stuffing, FLAG};
use log::{debug, trace};
use std::io::{Result, Seek, Write};

pub trait AshWrite: Write {
    /// Writes an ASH [`Frame`].
    ///
    /// # Arguments
    /// * `buffer` The buffer used for output buffering.
    ///
    /// # Errors
    /// Returns an [`std::io::Error`] if any I/O errors occur.
    fn write_frame<F>(&mut self, frame: &F, buffer: &mut FrameBuffer) -> Result<()>
    where
        F: Frame,
        for<'a> &'a F: IntoIterator<Item = u8>,
    {
        debug!("Writing frame: {frame}");
        trace!("{frame:#04X?}");
        buffer.rewind()?;
        buffer.extend(frame.into_iter().stuff())?;
        buffer.write_all(&[FLAG])?;
        self.write_all(buffer)
    }
}

impl<T> AshWrite for T where T: Write {}
