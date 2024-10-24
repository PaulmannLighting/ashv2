//! Asynchronous Serial Host version 2 (`ASHv2`)
//!
//! This library implements the Asynchronous Serial Host version 2, `ASHv2` for short.  
//! You can find the protocol's definition on [siliconlabs.com](https://www.silabs.com/documents/public/user-guides/ug101-uart-gateway-protocol-reference.pdf).
//!
//! This library is free software and is not affiliated with Silicon Labs.

pub use baud_rate::BaudRate;
pub use frames::Frames;
pub use serial_port::open;
pub use stream::Stream;
pub use transceiver::Transceiver;
pub use types::{Payload, MAX_PAYLOAD_SIZE, MIN_PAYLOAD_SIZE};
pub use utils::{make_pair, HexSlice};

mod baud_rate;
mod code;
mod crc;
mod frame;
mod frame_buffer;
mod frames;
mod packet;
mod protocol;
mod request;
mod serial_port;
mod status;
mod stream;
mod transceiver;
mod types;
mod utils;

const VERSION: u8 = 0x02;
