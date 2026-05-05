//! Asynchronous Serial Host version 2 (`ASHv2`)
//!
//! This library implements the Asynchronous Serial Host version 2, `ASHv2` for short.
//!
//! You can find the protocol's definition on [siliconlabs.com](https://docs.silabs.com/zigbee/latest/uart-gateway-protocol-reference/).
//!
//! This library is free software and is not affiliated with Silicon Labs.
#![deny(unsafe_code)]

use const_env::env_item;

pub use self::actor::{Actor, Proxy, Tasks};
pub use self::baud_rate::BaudRate;
pub use self::serial_port::{FlowControl, SerialPort, TryCloneNative, open};
pub use self::types::Payload;

/// Maximum payload size in bytes.
#[env_item("ASHV2_MAX_PAYLOAD_SIZE")]
pub const MAX_PAYLOAD_SIZE: usize = 128;

#[env_item("ASHV2_T_RSTACK_MAX_MILLIS")]
const T_RSTACK_MAX_MILLIS: u64 = 3200;

/// The amount of maximum unacknowledged frames that the NCP (or Host) can hold.
/// Also amounts to the so-called *sliding window size*.
#[env_item("ASHV2_TX_K")]
const TX_K: usize = 5;

#[env_item("ASHV2_T_RX_ACK_MAX_MILLIS")]
const T_RX_ACK_MAX_MILLIS: u64 = 3200;

#[env_item("ASHV2_REQUEUE_DELAY_MILLIS")]
const REQUEUE_DELAY_MILLIS: u64 = 100;

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
