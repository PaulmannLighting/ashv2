mod baud_rate;
mod code;
pub mod error;
pub mod frame;
pub mod packet;
pub mod protocol;
mod serial_port;

use code::Code;
use crc::{Crc, CRC_16_IBM_3740};
pub use error::Error;
pub const CRC: Crc<u16> = Crc::<u16>::new(&CRC_16_IBM_3740);
pub use baud_rate::BaudRate;
pub use protocol::Host;
pub use serial_port::open;
