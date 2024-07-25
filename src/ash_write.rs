use std::io::{Result, Write};

use log::{debug, trace};

use crate::frame::Frame;
use crate::protocol::{Stuff, FLAG};

pub trait AshWrite: Write {
    /// Writes an ASH [`Frame`].
    ///
    /// # Errors
    /// Returns an [`Error`](std::io::Error) if any I/O error occurs.
    fn write_frame<F>(&mut self, frame: &F) -> Result<()>
    where
        F: Frame;
}

impl<T> AshWrite for T
where
    T: Write,
{
    fn write_frame<F>(&mut self, frame: &F) -> Result<()>
    where
        F: Frame,
    {
        debug!("Writing frame: {frame}");
        trace!("{frame:#04X?}");

        for byte in frame.bytes().as_ref().iter().copied().stuff() {
            self.write_all(&[byte])?;
        }

        self.write_all(&[FLAG])?;
        self.flush()
    }
}
