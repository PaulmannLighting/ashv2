use crate::protocol::{Stuffing, FLAG};
use std::io::{Result, Write};

pub trait AshWrite: Write {
    /// Writes an ASH [`Frame`](crate::frame::Frame).
    ///
    /// # Arguments
    /// * `buffer` The buffer used for output buffering.
    ///
    /// # Errors
    /// Returns an [`std::io::Error`] if any I/O errors occur.
    fn write_frame<F>(&mut self, frame: F, buffer: &mut Vec<u8>) -> Result<()>
    where
        F: IntoIterator<Item = u8>,
    {
        buffer.clear();
        buffer.extend(frame.into_iter().stuff());
        buffer.push(FLAG);
        self.write_all(buffer)
    }
}

impl<T> AshWrite for T where T: Write {}
