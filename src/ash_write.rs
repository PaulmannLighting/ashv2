use crate::frame::Frame;
use crate::packet::FrameBuffer;
use crate::protocol::{Stuffing, FLAG};
use log::{debug, trace};
use std::io::{Error, ErrorKind, Result, Write};
use std::iter::once;

pub trait AshWrite: Write {
    /// Writes an ASH [`Frame`].
    ///
    /// # Arguments
    /// * `buffer` The buffer used for output buffering.
    ///
    /// # Errors
    /// Returns an [`Error`] if any I/O error occurs.
    fn write_frame<F>(&mut self, frame: &F, buffer: &mut FrameBuffer) -> Result<()>
    where
        F: Frame,
        for<'a> &'a F: IntoIterator<Item = u8>,
    {
        debug!("Writing frame: {frame}");
        trace!("{frame:#04X?}");
        buffer.clear();

        for byte in frame.into_iter().stuff().chain(once(FLAG)) {
            buffer
                .push(byte)
                .map_err(|_| Error::new(ErrorKind::OutOfMemory, "Buffer overflow."))?;
        }

        trace!("Buffer: {:#04X?}", buffer);
        self.write_all(buffer)
    }
}

impl<T> AshWrite for T where T: Write {}
