use std::io::{Result, Write};

use log::{debug, trace};

use crate::frame::Frame;
use crate::protocol::{Stuff, FLAG};

pub trait AshWrite: Write {
    /// Writes an ASH [`Frame`].
    ///
    /// # Arguments
    /// * `buffer` The buffer used for output buffering.
    ///
    /// # Errors
    /// Returns an [`Error`] if any I/O error occurs.
    fn write_frame<F>(&mut self, frame: &F) -> Result<()>
    where
        F: Frame,
        for<'a> &'a F: IntoIterator<Item = u8>;
}

impl<T> AshWrite for T
where
    T: Write,
{
    fn write_frame<F>(&mut self, frame: &F) -> Result<()>
    where
        F: Frame,
        for<'a> &'a F: IntoIterator<Item = u8>,
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
