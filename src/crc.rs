use crc::{Crc, CRC_16_IBM_3740};

/// CRC-16-IBM-3740 checksum function.
pub const CRC: Crc<u16> = Crc::<u16>::new(&CRC_16_IBM_3740);

/// A trait for object that can be validated by a CRC checksum.
pub trait Validate {
    /// Returns the CRC checksum.
    fn crc(&self) -> u16;

    /// Calculates the CRC checksum of the frame data.
    fn calculate_crc(&self) -> u16;

    /// Determines whether the CRC checksum is valid.
    fn is_crc_valid(&self) -> bool {
        self.crc() == self.calculate_crc()
    }
}
