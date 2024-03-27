use crate::packet::{MAX_PAYLOAD_SIZE, MIN_PAYLOAD_SIZE};
use crate::Error;
use itertools::{IntoChunks, Itertools};

pub trait AshChunks: IntoIterator<Item = u8> + Sized
where
    <Self as IntoIterator>::IntoIter: ExactSizeIterator,
{
    /// Return an iterator over chunks that fit into ASH data frames.
    ///
    /// # Errors
    /// Returns an [`Error`] if the bytes cannot be distributed across chunks of valid sizes.
    fn ash_chunks(self) -> Result<IntoChunks<Self::IntoIter>, Error> {
        let iterator = self.into_iter();
        let mut frame_size = MAX_PAYLOAD_SIZE;

        loop {
            if iterator.len() % frame_size == 0 || iterator.len() % frame_size >= MIN_PAYLOAD_SIZE {
                return Ok(iterator.chunks(frame_size));
            }

            frame_size = frame_size
                .checked_sub(1)
                .ok_or_else(|| Error::CannotFindViableChunkSize(iterator.len()))?;
        }
    }
}

impl<T> AshChunks for T
where
    T: IntoIterator<Item = u8>,
    <T as IntoIterator>::IntoIter: ExactSizeIterator,
{
}

#[cfg(test)]
mod tests {
    use super::AshChunks;
    use crate::protocol::ash_chunks::{MAX_PAYLOAD_SIZE, MIN_PAYLOAD_SIZE};
    use itertools::Itertools;

    #[test]
    fn test_ash_chunks() {
        let bytes = (u8::MIN..=u8::MAX).collect_vec();

        for chunk in &bytes
            .into_iter()
            .ash_chunks()
            .expect("Chunks should always be able to be distributed.")
        {
            let chunk = chunk.collect_vec();
            assert_eq!(
                chunk.len(),
                chunk.len().clamp(MIN_PAYLOAD_SIZE, MAX_PAYLOAD_SIZE)
            );
        }
    }
}
