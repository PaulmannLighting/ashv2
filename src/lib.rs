//! Asynchronous Serial Host version 2 (`ASHv2`)
//!
//! This library implements the Asynchronous Serial Host version 2, `ASHv2` for short.  
//! You can find the protocol's definition on [siliconlabs.com](https://www.silabs.com/documents/public/user-guides/ug101-uart-gateway-protocol-reference.pdf).
//!
//! This library is free software and is not affiliated with Silicon Labs.

use crc::{Crc, CRC_16_IBM_3740};

pub use baud_rate::BaudRate;
use code::Code;
pub use error::Error;
pub use frame_buffer::FrameBuffer;
pub use host::Host;
pub use protocol::{Event, HandleResult, Handler, Response};
pub use serial_port::open;

mod ash_read;
mod ash_write;
mod baud_rate;
mod code;
mod error;
mod frame;
mod frame_buffer;
mod host;
mod packet;
mod protocol;
mod request;
mod response;
mod serial_port;
mod transceiver;
mod util;
mod wrapping_u3;

const CRC: Crc<u16> = Crc::<u16>::new(&CRC_16_IBM_3740);
const VERSION: u8 = 0x02;

/// Result type with [`Error`].
pub type Result<T> = std::result::Result<T, Error>;
