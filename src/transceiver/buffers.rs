use super::transmission::Transmission;
use crate::transceiver::constants::TX_K;
use crate::types::FrameBuffer;

/// Buffers used by the transceiver.
#[derive(Debug, Default)]
pub struct Buffers {
    pub frame: FrameBuffer,
    pub transmissions: heapless::Vec<Transmission, TX_K>,
}

impl Buffers {
    /// Resets the transceiver buffers.
    pub fn clear(&mut self) {
        self.frame.clear();
        self.transmissions.clear();
    }
}
