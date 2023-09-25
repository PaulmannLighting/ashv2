use crate::{CRC, FLAG_BYTE};

pub trait Frame {
    /// Returns the frame's header.
    fn header(&self) -> u8;

    /// Returns the frame's payload.
    ///
    /// This is optional, since not all frames contain payload.
    fn payload(&self) -> Option<Vec<u8>>;

    /// Returns the CRC checksum.
    fn crc(&self) -> u16;

    /// Returns the flag byte.
    fn flag(&self) -> u8;

    /// Determines whether the header of the frame is valid.
    fn is_header_valid(&self) -> bool;

    /// Determines whether the flag byte is valid.
    fn is_flag_valid(&self) -> bool {
        self.flag() == FLAG_BYTE
    }

    /// Determines whether the CRC checksum is valid.
    fn is_crc_valid(&self) -> bool {
        self.crc() == self.calculate_crc()
    }

    /// Calculates the CRC checksum of the frame data.
    fn calculate_crc(&self) -> u16 {
        let mut buffer;

        if let Some(payload) = self.payload() {
            buffer = Vec::with_capacity(payload.len() + 1);
            buffer.push(self.header());
            buffer.extend_from_slice(&payload);
        } else {
            buffer = vec![self.header()];
        }

        CRC.checksum(&buffer)
    }

    /// Determines whether the frame is valid.
    fn is_valid(&self) -> bool {
        self.is_header_valid() && self.is_crc_valid() && self.is_flag_valid()
    }
}
