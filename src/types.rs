use crate::packet::Data;

/// In the wost-case, all frame bytes are stuffed (*2) and we append the FLAG byte (+1).
pub const MAX_FRAME_SIZE: usize = Data::BUFFER_SIZE * 2 + 1;

/// A stack-allocated buffer that can hold an `ASHv2` frame up to its maximum size with stuffing.
pub type FrameVec = heapless::Vec<u8, MAX_FRAME_SIZE>;

/// The minimum payload size of an `ASHv2` `DATA` frame.
pub const MIN_PAYLOAD_SIZE: usize = Data::MIN_PAYLOAD_SIZE;

/// The maximum payload size of an `ASHv2` `DATA` frame.
pub const MAX_PAYLOAD_SIZE: usize = Data::MAX_PAYLOAD_SIZE;

/// A stack-allocated buffer that can hold payload for an `ASHv2` `DATA` frame up to its maximum size.
pub type Payload = heapless::Vec<u8, MAX_PAYLOAD_SIZE>;
