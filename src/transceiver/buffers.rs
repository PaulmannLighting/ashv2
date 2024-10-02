use super::retransmit::Retransmit;
use crate::wrapping_u3::WrappingU3;
use crate::{FrameBuffer, Transceiver};
use log::trace;

/// Buffers used by the transceiver.
#[derive(Debug, Default)]
pub struct Buffers {
    pub(super) frame: FrameBuffer,
    pub(super) retransmits: heapless::Vec<Retransmit, { Transceiver::ACK_TIMEOUTS }>,
    pub(super) response: Vec<u8>,
}

impl Buffers {
    /// Resets the transceiver buffers.
    pub(super) fn clear(&mut self) {
        self.frame.clear();
        self.retransmits.clear();
        self.response.clear();
    }

    pub(in crate::transceiver) fn ack_sent_packets(&mut self, ack_num: WrappingU3) {
        trace!("Handling ACK: {ack_num}");
        while let Some(retransmit) = self
            .retransmits
            .iter()
            .position(|retransmit| retransmit.frame_num() + 1 == ack_num)
            .map(|index| self.retransmits.remove(index))
        {
            trace!("ACKed packet #{}", retransmit.into_data().frame_num());
        }
    }
}
