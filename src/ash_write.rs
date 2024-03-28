use std::io::{Result, Write};

use log::{debug, trace};

use crate::frame::Frame;
use crate::protocol::{Stuff, FLAG};

pub trait AshWrite {
    /// Writes an ASH [`Frame`].
    ///
    /// # Arguments
    /// * `buffer` The buffer used for output buffering.
    ///
    /// # Errors
    /// Returns an [`Error`] if any I/O error occurs.
    fn write_to<W>(&self, writer: &mut W) -> Result<()>
    where
        W: Write;
}

impl<T> AshWrite for T
where
    T: Frame,
    for<'a> &'a T: IntoIterator<Item = u8>,
{
    fn write_to<W>(&self, writer: &mut W) -> Result<()>
    where
        W: Write,
    {
        debug!("Writing frame: {self}");
        trace!("{self:#04X?}");

        for byte in self.into_iter().stuff() {
            writer.write_all(&[byte])?;
        }

        writer.write_all(&[FLAG])
    }
}
