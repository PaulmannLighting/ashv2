use super::sent_data::SentData;
use crate::frame_buffer::FrameBuffer;
use crate::packet::Data;
use crate::protocol::Mask;
use crate::transceiver::constants::ACK_TIMEOUTS;
use log::trace;

/// Buffers used by the transceiver.
#[derive(Debug, Default)]
pub struct Buffers {
    pub(super) frame: FrameBuffer,
    pub(super) sent_data: heapless::Vec<SentData, ACK_TIMEOUTS>,
    pub(super) response: Vec<u8>,
}

impl Buffers {
    /// Resets the transceiver buffers.
    pub(super) fn clear(&mut self) {
        self.frame.clear();
        self.sent_data.clear();
        self.response.clear();
    }

    /// Extends the response buffer with the given data.
    pub fn extend_response(&mut self, mut payload: heapless::Vec<u8, { Data::MAX_PAYLOAD_SIZE }>) {
        payload.mask();
        trace!("Extending response buffer with: {:#04X?}", payload);
        self.response.extend_from_slice(&payload);
        trace!("Response buffer is now: {:#04X?}", self.response);
    }
}
