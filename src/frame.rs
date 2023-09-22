use crate::{CRC, FLAG_BYTE};

pub trait Frame {
    fn header(&self) -> u8;
    fn payload(&self) -> Option<Vec<u8>>;
    fn crc(&self) -> u16;
    fn flag(&self) -> u8;
    fn is_header_valid(&self) -> bool;

    fn is_flag_valid(&self) -> bool {
        self.flag() == FLAG_BYTE
    }

    fn is_crc_valid(&self) -> bool {
        self.crc() == self.calculate_crc()
    }

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

    fn is_valid(&self) -> bool {
        self.is_header_valid() && self.is_crc_valid() && self.is_flag_valid()
    }
}
