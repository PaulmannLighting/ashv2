//! Asynchronous Serial Host version 2 (`ASHv2`)
//!
//! This library implement the Asynchronous Serial Host version 2, `ASHv2` for short.
//! You can find the protocol's definition on [siliconlabs.com](https://www.silabs.com/documents/public/user-guides/ug101-uart-gateway-protocol-reference.pdf).
//!
//! This library is free software and is not affiliated with Silicon Labs.

use crc::{Crc, CRC_16_IBM_3740};

pub use baud_rate::BaudRate;
use code::Code;
pub use error::Error;
pub use host::Host;
pub use packet::FrameBuffer;
pub use protocol::{Event, HandleResult, Handler, Response};
pub use serial_port::open;

mod ash_read;
mod ash_write;
mod baud_rate;
mod code;
mod error;
mod frame;
mod host;
mod packet;
mod protocol;
mod serial_port;
mod util;

const CRC: Crc<u16> = Crc::<u16>::new(&CRC_16_IBM_3740);
const VERSION: u8 = 0x02;
