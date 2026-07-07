use std::io::Result;

use bytes::Bytes;
use tokio::io::AsyncRead;
use tokio_stream::StreamExt;
use tokio_util::io::ReaderStream;

#[derive(Debug)]
pub struct AsyncBufStream<T> {
    reader: ReaderStream<T>,
    buffer: <Bytes as IntoIterator>::IntoIter,
}

impl<T> AsyncBufStream<T>
where
    T: AsyncRead,
{
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
