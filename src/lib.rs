//! Asynchronous Serial Host version 2 (`ASHv2`)
//!
//! This library implements the Asynchronous Serial Host version 2, `ASHv2` for short.  
//! You can find the protocol's definition on [siliconlabs.com](https://www.silabs.com/documents/public/user-guides/ug101-uart-gateway-protocol-reference.pdf).
//!
//! This library is free software and is not affiliated with Silicon Labs.

pub use any_sender::AnySender;
pub use async_ash::AsyncAsh;
pub use baud_rate::BaudRate;
pub use request::Request;
pub use serial_port::open;
pub use sync_ash::SyncAsh;
pub use transceiver::Transceiver;

mod any_sender;
mod async_ash;
mod baud_rate;
mod code;
mod crc;
mod frame;
mod frame_buffer;
mod packet;
mod protocol;
mod request;
mod serial_port;
mod status;
mod sync_ash;
mod transceiver;
mod wrapping_u3;

const VERSION: u8 = 0x02;
