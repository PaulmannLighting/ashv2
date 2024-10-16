//! Asynchronous Serial Host version 2 (`ASHv2`)
//!
//! This library implements the Asynchronous Serial Host version 2, `ASHv2` for short.  
//! You can find the protocol's definition on [siliconlabs.com](https://www.silabs.com/documents/public/user-guides/ug101-uart-gateway-protocol-reference.pdf).
//!
//! This library is free software and is not affiliated with Silicon Labs.

pub use ash_framed::AshFramed;
pub use baud_rate::BaudRate;
pub use callbacks::Callbacks;
pub use request::Request;
pub use serial_port::open;
pub use transceiver::Transceiver;
pub use types::{Payload, MAX_PAYLOAD_SIZE};
pub use utils::{make_pair, HexSlice};

mod ash_framed;
mod baud_rate;
mod callbacks;
mod code;
mod crc;
mod frame;
mod packet;
mod protocol;
mod request;
mod serial_port;
mod status;
mod transceiver;
mod types;
mod utils;

const VERSION: u8 = 0x02;
