mod code;
pub mod frame;
pub mod packet;

pub use code::Code;
use crc::{Crc, CRC_16_IBM_3740};
pub use frame::Frame;

pub const CRC: Crc<u16> = Crc::<u16>::new(&CRC_16_IBM_3740);
pub const FLAG_BYTE: u8 = 0x7E;
