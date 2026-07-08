//! Byte-oriented stream adapter for async readers.
//!
//! [`AsyncBufStream`] wraps Tokio's chunk-oriented [`ReaderStream`] and exposes an async
//! `next` method that yields one byte at a time. The receiver uses this to parse `ASHv2`
//! control bytes and frame boundaries without managing partially consumed byte chunks itself.

use std::io::Result;

use bytes::Bytes;
use tokio::io::AsyncRead;
use tokio_stream::StreamExt;
use tokio_util::io::ReaderStream;

/// Converts an [`AsyncRead`] value into a byte-at-a-time async stream.
///
/// The wrapper keeps the current chunk returned by [`ReaderStream`] and drains it before
/// polling the underlying reader for more data.
#[derive(Debug)]
pub struct AsyncBufStream<T> {
    /// Chunk stream produced from the wrapped reader.
    reader: ReaderStream<T>,
    /// Iterator over the currently buffered chunk.
    buffer: <Bytes as IntoIterator>::IntoIter,
}

impl<T> AsyncBufStream<T>
where
    T: AsyncRead,
{
    /// Create a new byte stream from an async reader.
    pub fn new(reader: T) -> Self {
        Self {
            reader: ReaderStream::new(reader),
            buffer: Bytes::new().into_iter(),
        }
    }
}

impl<T> AsyncBufStream<T>
where
    T: AsyncRead + Unpin,
{
    /// Return the next byte from the stream.
    ///
    /// The method returns `None` when the underlying reader reaches EOF.
    ///
    /// # Errors
    ///
    /// Returns an error if the wrapped reader fails while fetching the next byte chunk.
    pub async fn next(&mut self) -> Option<Result<u8>> {
        loop {
            if let Some(byte) = self.buffer.next() {
                return Some(Ok(byte));
            }

            match self.reader.next().await? {
                Ok(bytes) => {
                    self.buffer = bytes.into_iter();
                }
                Err(error) => return Some(Err(error)),
            }
        }
    }
}
