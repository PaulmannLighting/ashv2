//! CRC checksum validation.

use crc::{CRC_16_IBM_3740, Crc};

/// CRC-16-IBM-3740 checksum function.
pub const CRC: Crc<u16> = Crc::<u16>::new(&CRC_16_IBM_3740);

/// A trait for checksum based data validation.
pub trait Validate: Sized {
    /// Returns the CRC checksum.
    fn crc(&self) -> u16;

    /// Calculates the CRC checksum of the frame data.
    fn calculate_crc(&self) -> u16;

    /// Validates whether the CRC checksum is valid.
    ///
    /// # Returns
    ///
    /// Returns `Self` if the checksum is valid.
    ///
    /// # Errors
    ///
    /// Returns the calculated CRC checksum if invalid.
    fn validate(self) -> Result<Self, u16> {
        let calculated_crc = self.calculate_crc();

        if self.crc() == calculated_crc {
            Ok(self)
        } else {
            Err(calculated_crc)
        }
    }
}
