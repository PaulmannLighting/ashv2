use super::retransmit::Retransmit;
use crate::packet::Data;
use crate::protocol::Mask;
use crate::transceiver::constants::ACK_TIMEOUTS;
use crate::FrameBuffer;

/// Buffers used by the transceiver.
#[derive(Debug, Default)]
pub struct Buffers {
    pub(super) frame: FrameBuffer,
    pub(super) retransmits: heapless::Vec<Retransmit, ACK_TIMEOUTS>,
    pub(super) response: Vec<u8>,
}

impl Buffers {
    /// Resets the transceiver buffers.
    pub(super) fn clear(&mut self) {
        self.frame.clear();
        self.retransmits.clear();
        self.response.clear();
    }

    /// Extends the response buffer with the given data.
    pub fn extend_response(&mut self, mut payload: heapless::Vec<u8, { Data::MAX_PAYLOAD_SIZE }>) {
        payload.mask();
        self.response.extend_from_slice(&payload);
    }
}
