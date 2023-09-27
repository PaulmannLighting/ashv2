use crate::CRC;

pub trait Frame
where
    for<'a> &'a Self: Into<Vec<u8>>,
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
        let bytes: Vec<u8> = self.into();
        // Exclude last two bytes which constitute the CRC checksum.
        CRC.checksum(&bytes[0..bytes.len() - 2])
    }

    /// Determines whether the frame is valid.
    fn is_valid(&self) -> bool {
        self.is_header_valid() && self.is_crc_valid()
    }
}
