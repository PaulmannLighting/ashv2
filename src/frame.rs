use std::fmt::{Debug, Display};

use crate::CRC;

pub trait Frame: Debug + Display {
    /// Returns the frame's header.
    fn header(&self) -> u8;

    /// Returns the CRC checksum.
    fn crc(&self) -> u16;

    /// Determines whether the header of the frame is valid.
    fn is_header_valid(&self) -> bool;

    /// Determines whether the CRC checksum is valid.
    fn is_crc_valid(&self) -> bool {
        self.crc() == self.calculate_crc()
    }

    /// Calculates the CRC checksum of the frame data.
    fn calculate_crc(&self) -> u16 {
        CRC.checksum(&[self.header()])
    }

    /// Determines whether the frame is valid.
    #[cfg(test)]
    fn is_valid(&self) -> bool {
        self.is_header_valid() && self.is_crc_valid()
    }

    fn bytes(&self) -> impl AsRef<[u8]>;
}
