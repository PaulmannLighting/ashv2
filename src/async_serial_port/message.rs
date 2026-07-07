use std::io::Result;

use bytes::{Bytes, BytesMut};
use tokio::sync::oneshot::Sender;

#[expect(variant_size_differences)]
pub enum Message {
    Write {
        buffer: Bytes,
        response: Sender<Result<()>>,
    },
    Read(Sender<Result<BytesMut>>),
    Flush(Sender<Result<()>>),
}
