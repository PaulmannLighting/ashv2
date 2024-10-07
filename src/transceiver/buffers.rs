use super::constants::TX_K;
use super::transmission::Transmission;
use crate::frame_buffer::FrameBuffer;

/// Buffers used by the transceiver.
#[derive(Debug, Default)]
pub struct Buffers {
    pub(super) frame: FrameBuffer,
    pub(super) transmissions: heapless::Vec<Transmission, TX_K>,
}

impl Buffers {
    /// Resets the transceiver buffers.
    pub(super) fn clear(&mut self) {
        self.frame.clear();
        self.transmissions.clear();
    }
}
