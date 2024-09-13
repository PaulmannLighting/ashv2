use crate::packet::Data;

// In the wost-case, all frame bytes are stuffed (*2) and we append the FLAG byte (+1).
const MAX_FRAME_SIZE: usize = (Data::METADATA_SIZE + Data::MAX_PAYLOAD_SIZE) * 2 + 1;

/// A stack-allocated buffer that can hold an `ASHv2` frame up to its maximum size.
pub type FrameBuffer = heapless::Vec<u8, MAX_FRAME_SIZE>;
