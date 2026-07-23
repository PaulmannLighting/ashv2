//! Asynchronous Serial Host version 2 (`ASHv2`)
//!
//! This library implements the Asynchronous Serial Host version 2, `ASHv2` for short.
//! It provides frame parsing and actor futures that manage the host-side link.
//!
//! # Transport API
//!
//! [`start`] accepts separate types implementing [`tokio::io::AsyncRead`] and
//! [`tokio::io::AsyncWrite`]. The caller opens, configures, and, when necessary, splits the
//! underlying transport before starting the actor futures. The core API has no dependency on
//! `serialport` or `async-serialport`.
//!
//! The returned [`Futures`] contains the transmitter and receiver futures. The caller must spawn
//! or otherwise poll both futures on an async runtime.
//!
//! # Termination
//!
//! The actor does not use a terminate message. Drop every clone of [`Handle`] to close the
//! outbound message queue. The transmitter drains queued messages and then terminates. It clears
//! the shared running state when it exits, which causes the receiver to terminate as well.
//! Continue polling or awaiting both actor futures until they complete.
//!
//! # EZSP integration
//!
//! The optional `ezsp` feature provides [`ezsp::Transmitter`] and [`ezsp::Receiver`] adapters.
//! [`ezsp::Transmitter`] is an alias for [`Handle`], which implements `ezsp::Transmit`.
//! [`ezsp::Receiver`] consumes the channel of inbound [`Payload`] values passed to [`start`] and
//! implements `ezsp::Receive`.
//!
//! You can find the protocol's definition on [siliconlabs.com](https://docs.silabs.com/zigbee/latest/uart-gateway-protocol-reference/).
//!
//! This library is free software and is not affiliated with Silicon Labs.
#![cfg_attr(docsrs, feature(doc_cfg))]
#![deny(unsafe_code)]

use const_env::env_item;

pub use self::actor::{Futures, Handle, start};
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

const VERSION: u8 = 0x02;

const SEQ_MASK: u8 = 0b0000_0111;

mod actor;
mod code;
#[cfg(feature = "ezsp")]
#[cfg_attr(docsrs, doc(cfg(feature = "ezsp")))]
pub mod ezsp;
mod frame;
mod hex_slice;
mod protocol;
mod status;
mod types;
mod validate;
