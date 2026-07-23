//! Optional adapters between the `ASHv2` payload API and typed EZSP frames.
//!
//! This module is available with the `ezsp` crate feature. [`Transmitter`] wraps an `ASHv2`
//! [`crate::Handle`] and implements `ezsp::Transmit`. [`Receiver`] consumes the inbound
//! [`crate::Payload`] channel and implements `ezsp::Receive`.

pub use self::receiver::Receiver;
pub use self::transmitter::Transmitter;

mod receiver;
mod transmitter;
