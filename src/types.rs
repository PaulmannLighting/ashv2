//! Common types used in the `ASHv2` protocol implementation.

use crate::MAX_PAYLOAD_SIZE;
use crate::frame::Data;

/// In the wost-case, all frame bytes are stuffed (*2) and we append the FLAG byte (+1).
pub const MAX_FRAME_SIZE: usize = Data::BUFFER_SIZE * 2 + 1;

/// A stack-allocated buffer that can hold bytes of an `ASHv2` frame up to its maximum size with stuffing.
pub type RawFrame = heapless::Vec<u8, MAX_FRAME_SIZE>;

/// A stack-allocated buffer that can hold payload for an `ASHv2` `DATA` frame up to its maximum size.
pub type Payload = heapless::Vec<u8, MAX_PAYLOAD_SIZE>;
