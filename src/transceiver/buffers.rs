use super::transmission::Transmission;
use crate::types::FrameBuffer;

/// Buffers used by the transceiver.
#[derive(Debug, Default)]
pub struct Buffers<const SLIDING_WINDOW_SIZE: usize> {
    pub frame: FrameBuffer,
    pub transmissions: heapless::Vec<Transmission, SLIDING_WINDOW_SIZE>,
}

impl<const SLIDING_WINDOW_SIZE: usize> Buffers<SLIDING_WINDOW_SIZE> {
    /// Resets the transceiver buffers.
    pub fn clear(&mut self) {
        self.frame.clear();
        self.transmissions.clear();
    }
}
