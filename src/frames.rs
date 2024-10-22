use crate::protocol::{Stuffing, FLAG};
use crate::types::FrameBuffer;
use crate::types::Payload;

/// A trait for iterating over `ASHv2` encoded frames.
pub trait Frames: Iterator<Item = u8> {
    /// Returns an iterator over the payload frames.
    fn frames(&mut self) -> PayloadIterator<&mut Self> {
        PayloadIterator {
            iterator: self,
            buffer: FrameBuffer::new(),
        }
    }
}

impl<T> Frames for T where T: Iterator<Item = u8> {}

pub struct PayloadIterator<T>
where
    T: Iterator<Item = u8>,
{
    iterator: T,
    buffer: FrameBuffer,
}

impl<T> Iterator for PayloadIterator<T>
where
    T: Iterator<Item = u8>,
{
    type Item = Payload;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            match self.iterator.next()? {
                FLAG => {
                    self.buffer.unstuff();
                    let mut payload = Payload::new();
                    payload.extend_from_slice(&self.buffer).ok()?;
                    self.buffer.clear();
                    return Some(payload);
                }
                other => {
                    self.buffer.push(other).ok()?;
                    continue;
                }
            }
        }
    }
}
