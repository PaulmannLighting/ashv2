use crate::packet::Data;
use crate::retransmit::Retransmit;
use crate::{FrameBuffer, Transceiver};

/// Buffers used by the transceiver.
#[derive(Debug, Default)]
pub struct Buffers {
    pub(super) frame: FrameBuffer,
    pub(super) payload: heapless::Vec<u8, { Data::MAX_PAYLOAD_SIZE }>,
    pub(super) retransmits: heapless::Vec<Retransmit, { Transceiver::ACK_TIMEOUTS }>,
    pub(super) response: Vec<u8>,
}

impl Buffers {
    /// Resets the transceiver buffers.
    pub(super) fn clear(&mut self) {
        self.frame.clear();
        self.payload.clear();
        self.retransmits.clear();
        self.response.clear();
    }
}
