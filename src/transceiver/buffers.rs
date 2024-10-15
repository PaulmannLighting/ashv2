use super::constants::SLIDING_WINDOW_SIZE;
use super::transmission::Transmission;
use crate::types::FrameBuffer;

/// Buffers used by the transceiver.
#[derive(Debug, Default)]
pub struct Buffers {
    pub frame: FrameBuffer,
    pub transmissions: heapless::Vec<Transmission, SLIDING_WINDOW_SIZE>,
}

impl Buffers {
    /// Resets the transceiver buffers.
    pub fn clear(&mut self) {
        self.frame.clear();
        self.transmissions.clear();
    }
}
