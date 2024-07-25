use std::io::{Result, Write};

use log::{debug, trace};

use crate::frame::Frame;
use crate::protocol::FLAG;

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
        self.write_all(&frame.stuffed())?;
        self.write_all(&[FLAG])?;
        self.flush()
    }
}
