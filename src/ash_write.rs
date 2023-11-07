use crate::frame::Frame;
use crate::packet::MAX_FRAME_SIZE;
use crate::protocol::{Stuffing, FLAG};
use log::{debug, trace};
use std::io::{ErrorKind, Result, Write};

pub trait AshWrite: Write {
    /// Writes an ASH [`Frame`].
    ///
    /// # Arguments
    /// * `buffer` The buffer used for output buffering.
    ///
    /// # Errors
    /// Returns an [`std::io::Error`] if any I/O error occurs.
    fn write_frame<F>(
        &mut self,
        frame: &F,
        buffer: &mut heapless::Vec<u8, MAX_FRAME_SIZE>,
    ) -> Result<()>
    where
        F: Frame,
        for<'a> &'a F: IntoIterator<Item = u8>,
    {
        debug!("Writing frame: {frame}");
        trace!("{frame:#04X?}");
        buffer.clear();
        buffer.extend(frame.into_iter().stuff());
        buffer
            .push(FLAG)
            .map_err(|_| std::io::Error::new(ErrorKind::OutOfMemory, "Buffer overflow."))?;
        trace!("Buffer: {:#04X?}", buffer);
        self.write_all(buffer)
    }
}

impl<T> AshWrite for T where T: Write {}
