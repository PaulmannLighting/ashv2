use std::fmt::{Debug, Display};

use crate::types::FrameVec;

/// A trait to represent a frame.
pub trait Frame: Debug + Display {
    /// Returns the CRC checksum.
    fn crc(&self) -> u16;

    /// Determines whether the CRC checksum is valid.
    fn is_crc_valid(&self) -> bool {
        self.crc() == self.calculate_crc()
    }

    /// Calculates the CRC checksum of the frame data.
    fn calculate_crc(&self) -> u16;

    /// Returns the frame as a byte slice.
    ///
    /// # Errors
    ///
    /// Returns an error if the frame buffer overflows.
    fn buffer(&self, buffer: &mut FrameVec) -> std::io::Result<()>;
}
