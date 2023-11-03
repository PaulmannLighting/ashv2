use crate::protocol::FLAG;
use std::io::{Result, Write};

pub trait AshWrite: Write {
    fn write_frame(&mut self, frame: &[u8]) -> Result<()> {
        self.write_all(frame)?;
        self.write_all(&[FLAG])
    }
}

impl<T> AshWrite for T where T: Write {}
