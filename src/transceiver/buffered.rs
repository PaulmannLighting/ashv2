use crate::transceiver::Transceiver;
use crate::FrameBuffer;

pub trait Buffered {
    /// Returns the frame buffer in read-only mode.
    fn buffer(&self) -> &FrameBuffer;

    /// Returns the frame buffer in read-write mode.
    fn buffer_mut(&mut self) -> &mut FrameBuffer;
}

impl Buffered for Transceiver {
    fn buffer(&self) -> &FrameBuffer {
        &self.frame_buffer
    }

    fn buffer_mut(&mut self) -> &mut FrameBuffer {
        &mut self.frame_buffer
    }
}
