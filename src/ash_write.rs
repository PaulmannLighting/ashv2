use std::io::{Error, ErrorKind, Result, Write};

use log::{debug, trace};

use crate::frame::Frame;
use crate::protocol::{Stuff, FLAG};

pub trait AshWrite: Write {
    /// Writes an ASH [`Frame`].
    ///
    /// # Errors
    /// Returns an [`Error`](Error) if any I/O error occurs.
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
        let mut buffer = frame.buffered();
        buffer.stuff();
        buffer
            .push(FLAG)
            .map_err(|_| Error::new(ErrorKind::OutOfMemory, "could not append flag byte"))?;
        trace!("Writing bytes: {buffer:#04X?}");
        self.write_all(&buffer)?;
        self.flush()
    }
}
