use super::retransmit::Retransmit;
use crate::packet::Data;
use crate::protocol::Mask;
use crate::transceiver::constants::ACK_TIMEOUTS;
use crate::wrapping_u3::WrappingU3;
use crate::FrameBuffer;
use log::{debug, trace};

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

    pub(in crate::transceiver) fn ack_sent_packets(&mut self, ack_num: WrappingU3) {
        trace!("Handling ACK: {ack_num}");
        while let Some(retransmit) = self
            .retransmits
            .iter()
            .position(|retransmit| retransmit.frame_num() + 1 == ack_num)
            .map(|index| self.retransmits.remove(index))
        {
            if let Ok(duration) = retransmit.elapsed() {
                debug!(
                    "ACKed packet #{} after {duration:?}",
                    retransmit.into_data().frame_num()
                );
            } else {
                debug!("ACKed packet #{}", retransmit.into_data().frame_num());
            }
        }
    }
}
