mod code;
pub mod error;
pub mod frame;
pub mod packet;
pub mod protocol;

pub use code::Code;
use crc::{Crc, CRC_16_IBM_3740};
pub use error::Error;
pub use frame::Frame;

pub const CRC: Crc<u16> = Crc::<u16>::new(&CRC_16_IBM_3740);
pub use protocol::Host;
