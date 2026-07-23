//! Optional adapters between the `ASHv2` payload API and typed EZSP frames.
//!
//! This module is available with the `ezsp` crate feature. [`Transmitter`] aliases
//! [`crate::Handle`], which implements `ezsp::Transmit`. [`Receiver`] consumes the inbound
//! [`crate::Payload`] channel and implements `ezsp::Receive`; the negotiated EZSP version is
//! supplied to each receive call by the EZSP layer.

pub use self::receiver::Receiver;
pub use self::transmitter::Transmitter;

mod receiver;
mod transmitter;
