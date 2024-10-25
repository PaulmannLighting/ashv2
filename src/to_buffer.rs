use crate::types::FrameVec;

/// A trait for objects that can be written to a frame buffer.
pub trait ToBuffer {
    /// Write the frame to the given buffer.
    ///
    /// # Errors
    ///
    /// Returns an error if the frame buffer overflows.
    fn buffer(&self, buffer: &mut FrameVec) -> std::io::Result<()>;
}
