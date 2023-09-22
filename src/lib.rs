pub mod packet;

use crc::{Crc, CRC_16_IBM_3740};

pub const CRC: Crc<u16> = Crc::<u16>::new(&CRC_16_IBM_3740);
pub const FLAG_BYTE: u8 = 0x7E;
