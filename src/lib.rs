use crc::{Crc, CRC_16_IBM_3740};

pub use ash_read::AshRead;
pub use ash_write::AshWrite;
pub use baud_rate::BaudRate;
use code::Code;
pub use error::Error;
pub use host::Host;
pub use protocol::{Event, HandleResult, Handler, Response};
pub use serial_port::open;

mod ash_read;
mod ash_write;
mod baud_rate;
mod code;
pub mod error;
pub mod frame;
mod host;
pub mod packet;
pub mod protocol;
mod serial_port;
mod util;

pub const CRC: Crc<u16> = Crc::<u16>::new(&CRC_16_IBM_3740);
