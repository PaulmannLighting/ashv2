use std::io::Result;
use std::sync::mpsc::{sync_channel, Receiver, SyncSender};

/// An incoming request.
#[derive(Debug)]
pub struct Request {
    pub(crate) payload: Box<[u8]>,
    pub(crate) response: SyncSender<Result<Box<[u8]>>>,
}

impl Request {
    #[must_use]
    pub(crate) fn new(payload: Box<[u8]>) -> (Self, Receiver<Result<Box<[u8]>>>) {
        let (response, rx) = sync_channel(1);
        (Self { payload, response }, rx)
    }
}
