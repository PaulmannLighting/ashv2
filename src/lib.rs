//! Asynchronous Serial Host version 2 (`ASHv2`)
//!
//! This library implements the Asynchronous Serial Host version 2, `ASHv2` for short.
//!
//! You can find the protocol's definition on [siliconlabs.com](https://www.silabs.com/documents/public/user-guides/ug101-uart-gateway-protocol-reference.pdf).
//!
//! This library is free software and is not affiliated with Silicon Labs.
#![deny(unsafe_code)]

pub use self::baud_rate::BaudRate;
pub use self::serial_port::{FlowControl, SerialPort, open};
pub use self::transceiver::Transceiver;
pub use self::types::{MAX_PAYLOAD_SIZE, MIN_PAYLOAD_SIZE, Payload};
pub use self::utils::HexSlice;
#[cfg(feature = "devel")]
pub use self::{
    code::Code,
    frame::{Ack, Data, Error, Frame, Nak, Rst, RstAck, headers},
    protocol::{ControlByte, Mask, Stuffing},
    status::Status,
    types::{MAX_FRAME_SIZE, RawFrame},
    utils::WrappingU3,
    validate::{CRC, Validate},
};

const VERSION: u8 = 0x02;

mod actor;
mod baud_rate;
mod code;
mod frame;
mod protocol;
mod serial_port;
mod status;
mod transceiver;
mod types;
mod utils;
mod validate;
