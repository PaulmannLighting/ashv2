//! Asynchronous Serial Host version 2 (`ASHv2`)
//!
//! This library implements the Asynchronous Serial Host version 2, `ASHv2` for short.
//!
//! You can find the protocol's definition on [siliconlabs.com](https://www.silabs.com/documents/public/user-guides/ug101-uart-gateway-protocol-reference.pdf).
//!
//! This library is free software and is not affiliated with Silicon Labs.
#![deny(unsafe_code)]

use const_env::env_item;

pub use self::actor::{Actor, Proxy};
pub use self::baud_rate::BaudRate;
pub use self::serial_port::{FlowControl, SerialPort, TryCloneNative, open};
pub use self::types::Payload;

/// Maximum payload size in bytes.
#[env_item("ASHV2_MAX_PAYLOAD_SIZE")]
pub const MAX_PAYLOAD_SIZE: usize = 128;

const VERSION: u8 = 0x02;

mod actor;
mod baud_rate;
mod code;
mod frame;
mod protocol;
mod serial_port;
mod status;
mod types;
mod utils;
mod validate;
