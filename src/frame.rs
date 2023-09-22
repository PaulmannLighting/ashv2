use crc::{Crc, CRC_16_IBM_3740};

const CRC: Crc<u16> = Crc::<u16>::new(&CRC_16_IBM_3740);

pub trait Frame {
    fn payload(&self) -> Vec<u8>;
    fn crc(&self) -> [u8; 2] {
        CRC.checksum(self.payload().as_slice()).to_be_bytes()
    }
}
