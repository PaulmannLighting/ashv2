//! Asynchronous Serial Host version 2 (`ASHv2`)
//!
//! This library implements the Asynchronous Serial Host version 2, `ASHv2` for short.  
//! You can find the protocol's definition on [siliconlabs.com](https://www.silabs.com/documents/public/user-guides/ug101-uart-gateway-protocol-reference.pdf).
//!
//! This library is free software and is not affiliated with Silicon Labs.
#![deny(unsafe_code)]

pub use baud_rate::BaudRate;
pub use serial_port::open;
pub use transceiver::Transceiver;
pub use types::{Payload, MAX_PAYLOAD_SIZE, MIN_PAYLOAD_SIZE};
pub use utils::HexSlice;

#[cfg(feature = "devel")]
pub use {
    code::Code,
    crc::{Validate, CRC},
    frame::{headers, Ack, Data, Error, Frame, Nak, Rst, RstAck},
    frame_buffer::FrameBuffer,
    protocol::{Mask, Stuffing, CANCEL, ESCAPE, FLAG, SUBSTITUTE, WAKE, X_OFF, X_ON},
    status::Status,
    to_buffer::ToBuffer,
    types::{FrameVec, MAX_FRAME_SIZE},
    utils::WrappingU3,
};

const VERSION: u8 = 0x02;

mod baud_rate;
mod code;
mod crc;
mod frame;
mod frame_buffer;
mod protocol;
mod serial_port;
mod status;
mod to_buffer;
mod transceiver;
mod types;
mod utils;
