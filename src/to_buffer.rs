//! Buffering of frames.

use crate::types::RawFrame;

/// A trait for objects that can be written to a frame buffer.
pub trait ToBuffer {
    /// Write the frame to the given buffer.
    ///
    /// # Errors
    ///
    /// Returns an [`std::io::Error`] if the frame buffer overflows.
    fn buffer(&self, buffer: &mut RawFrame) -> std::io::Result<()>;
}
