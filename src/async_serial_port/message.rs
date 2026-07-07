use std::io::Result;

use bytes::{Bytes, BytesMut};
use tokio::sync::oneshot::Sender;

pub enum Message {
    Write {
        buffer: Bytes,
        response: Sender<Result<()>>,
    },
    Read(Sender<Result<BytesMut>>),
    Flush(Sender<Result<()>>),
}
