use std::io::{Result, Write};

use log::{debug, trace};

use crate::frame::Frame;
use crate::protocol::{Stuff, FLAG};

pub trait AshWrite: Write {
    /// Writes an ASH [`Frame`].
    ///
    /// # Errors
    /// Returns an [`Error`](std::io::Error) if any I/O error occurs.
    fn write_frame<'frame, F>(&mut self, frame: &'frame F) -> Result<()>
    where
        F: Frame,
        &'frame F: IntoIterator<Item = u8>;
}

impl<T> AshWrite for T
where
    T: Write,
{
    fn write_frame<'frame, F>(&mut self, frame: &'frame F) -> Result<()>
    where
        F: Frame,
        &'frame F: IntoIterator<Item = u8>,
    {
        debug!("Writing frame: {frame}");
        trace!("{frame:#04X?}");

        for byte in frame.into_iter().stuff() {
            self.write_all(&[byte])?;
        }

        self.write_all(&[FLAG])?;
        self.flush()
    }
}
