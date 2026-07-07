use std::io::Result;

use tokio::sync::oneshot::Sender;

pub enum Message {
    Write {
        buffer: Box<[u8]>,
        response: Sender<Result<()>>,
    },
    Read {
        buffer: Box<[u8]>,
        response: Sender<Result<usize>>,
    },
    Flush(Sender<Result<()>>),
}
