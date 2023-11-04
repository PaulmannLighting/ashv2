use crate::CRC;
use std::fmt::{Debug, Display};

pub trait Frame: Debug + Display
where
    for<'a> &'a Self: IntoIterator<Item = u8>,
{
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
    fn is_valid(&self) -> bool {
        self.is_header_valid() && self.is_crc_valid()
    }
}
