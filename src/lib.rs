//! Asynchronous Serial Host version 2 (`ASHv2`)
//!
//! This library implements the Asynchronous Serial Host version 2, `ASHv2` for short.
//!
//! You can find the protocol's definition on [siliconlabs.com](https://www.silabs.com/documents/public/user-guides/ug101-uart-gateway-protocol-reference.pdf).
//!
//! This library is free software and is not affiliated with Silicon Labs.
#![deny(unsafe_code)]

pub use baud_rate::BaudRate;
#[cfg(feature = "serialport")]
pub use serial_port::open;
pub use transceiver::Transceiver;
pub use types::{MAX_PAYLOAD_SIZE, MIN_PAYLOAD_SIZE, Payload};
pub use utils::HexSlice;
#[cfg(feature = "devel")]
pub use {
    code::Code,
    frame::{Ack, Data, Error, Frame, Nak, Rst, RstAck, headers},
    frame_buffer::FrameBuffer,
    protocol::{CANCEL, ESCAPE, FLAG, Mask, SUBSTITUTE, Stuffing, WAKE, X_OFF, X_ON},
    status::Status,
    to_buffer::ToBuffer,
    types::{MAX_FRAME_SIZE, RawFrame},
    utils::WrappingU3,
    validate::{CRC, Validate},
};

const VERSION: u8 = 0x02;

mod baud_rate;
mod code;
mod frame;
mod frame_buffer;
mod protocol;
mod serial_port;
mod status;
mod to_buffer;
mod transceiver;
mod types;
mod utils;
mod validate;
