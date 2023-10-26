use crate::Error;
use itertools::{IntoChunks, Itertools};

const FRAME_MIN_SIZE: usize = 3;
const FRAME_MAX_SIZE: usize = 128;

pub trait AshChunks: IntoIterator<Item = u8>
where
    <Self as IntoIterator>::IntoIter: ExactSizeIterator,
    Self: Sized,
{
    /// Return an iterator over chunks that fit into ASH data frames.
    ///
    /// # Errors
    /// Returns an [`Error`] if the bytes cannot be distributed across chunks of valid sizes.
    fn ash_chunks(self) -> Result<IntoChunks<Self::IntoIter>, Error> {
        let iterator = self.into_iter();
        let mut frame_size = FRAME_MAX_SIZE;

        loop {
            if iterator.len() % FRAME_MAX_SIZE == 0
                || iterator.len() % FRAME_MAX_SIZE >= FRAME_MIN_SIZE
            {
                return Ok(iterator.chunks(frame_size));
            }

            frame_size = frame_size
                .checked_sub(1)
                .ok_or_else(|| Error::TooFewData(iterator.len()))?;
        }
    }
}

impl<T> AshChunks for T
where
    T: IntoIterator<Item = u8>,
    <T as IntoIterator>::IntoIter: ExactSizeIterator,
{
}
