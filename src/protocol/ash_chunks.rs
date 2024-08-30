use itertools::{IntoChunks, Itertools};

use crate::packet::Data;
use crate::Error;

pub trait AshChunks: Iterator<Item = u8> + ExactSizeIterator + Sized
where
    <Self as IntoIterator>::IntoIter: ExactSizeIterator,
{
    /// Return an iterator over chunks that fit into ASH data frames.
    ///
    /// # Errors
    /// Returns an [`Error`] if the bytes cannot be distributed across chunks of valid sizes.
    fn ash_chunks(self) -> Result<IntoChunks<Self>, Error>;
}

impl<T> AshChunks for T
where
    T: Iterator<Item = u8> + ExactSizeIterator,
{
    fn ash_chunks(self) -> Result<IntoChunks<Self>, Error> {
        if self.len() < Data::MIN_PAYLOAD_SIZE {
            return Err(Error::CannotFindViableChunkSize(self.len()));
        }

        if self.len() <= Data::MAX_PAYLOAD_SIZE || self.len() % Data::MAX_PAYLOAD_SIZE == 0 {
            return Ok(self.chunks(Data::MAX_PAYLOAD_SIZE));
        }

        for frame_size in (Data::MIN_PAYLOAD_SIZE..=Data::MAX_PAYLOAD_SIZE).rev() {
            let remainder = self.len() % frame_size;

            if remainder == 0 || remainder >= Data::MIN_PAYLOAD_SIZE {
                return Ok(self.chunks(frame_size));
            }
        }

        Err(Error::CannotFindViableChunkSize(self.len()))
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use itertools::Itertools;

    use crate::packet::Data;
    use crate::Error;

    use super::AshChunks;

    #[test]
    fn test_too_few_ash_chunks() {
        let bytes = (1..Data::MIN_PAYLOAD_SIZE)
            .map(|num| u8::try_from(num).expect("Number should be a valid u8"))
            .collect_vec();
        let chunks = bytes.into_iter().ash_chunks();
        assert!(chunks.is_err());
    }

    #[test]
    fn test_ash_chunks_max_size() {
        let bytes = (1..=Data::MAX_PAYLOAD_SIZE)
            .map(|num| u8::try_from(num).expect("Number should be a valid u8"))
            .collect_vec();
        let chunks: Vec<Vec<_>> = bytes
            .iter()
            .copied()
            .ash_chunks()
            .expect("Chunks should be valid.")
            .into_iter()
            .map(Iterator::collect)
            .collect();
        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0].len(), bytes.len());
    }

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
                chunk
                    .len()
                    .clamp(Data::MIN_PAYLOAD_SIZE, Data::MAX_PAYLOAD_SIZE)
            );
        }
    }

    #[test]
    fn test_min_payload_size() {
        let bytes = vec![0; Data::MIN_PAYLOAD_SIZE];
        let chunks: Vec<_> = chunks(&bytes).unwrap();
        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0].len(), Data::MIN_PAYLOAD_SIZE);
    }

    #[test]
    fn test_max_payload_size() {
        let bytes = vec![0; Data::MAX_PAYLOAD_SIZE];
        let chunks: Vec<_> = chunks(&bytes).unwrap();
        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0].len(), Data::MAX_PAYLOAD_SIZE);
    }

    #[test]
    fn test_mid_payload_size() {
        let mid_size = (Data::MIN_PAYLOAD_SIZE + Data::MAX_PAYLOAD_SIZE) / 2;
        let bytes = vec![0; mid_size];
        let chunks: Vec<_> = chunks(&bytes).unwrap();
        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0].len(), mid_size);
    }

    #[test]
    fn test_large_even_payload_size() {
        let size = Data::MAX_PAYLOAD_SIZE * 2;
        let bytes = vec![0; size];
        let chunks: Vec<_> = chunks(&bytes).unwrap();
        assert_eq!(chunks.len(), 2);
        assert_eq!(chunks[0].len(), Data::MAX_PAYLOAD_SIZE);
        assert_eq!(chunks[1].len(), Data::MAX_PAYLOAD_SIZE);
    }

    #[test]
    fn test_large_odd_payload_size() {
        let size = Data::MAX_PAYLOAD_SIZE * 2 + Data::MIN_PAYLOAD_SIZE;
        let bytes = vec![0; size];
        let chunks: Vec<_> = chunks(&bytes).unwrap();
        assert_eq!(chunks.len(), 3);
        assert_eq!(chunks[0].len(), Data::MAX_PAYLOAD_SIZE);
        assert_eq!(chunks[1].len(), Data::MAX_PAYLOAD_SIZE);
        assert_eq!(chunks[2].len(), Data::MIN_PAYLOAD_SIZE);
    }

    fn chunks(bytes: &[u8]) -> Result<Vec<Vec<u8>>, Error> {
        bytes
            .iter()
            .copied()
            .ash_chunks()
            .map(|chunks| chunks.into_iter().map(Iterator::collect).collect())
    }
}
