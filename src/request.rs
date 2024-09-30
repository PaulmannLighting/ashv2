use std::io::Result;
use std::sync::mpsc::{channel, Receiver, Sender};

/// An incoming request.
#[derive(Debug)]
pub struct Request {
    pub(crate) payload: Box<[u8]>,
    pub(crate) response: Sender<Result<Box<[u8]>>>,
}

impl Request {
    #[must_use]
    pub fn new(payload: Box<[u8]>) -> (Self, Receiver<Result<Box<[u8]>>>) {
        let (response, rx) = channel();
        (Self { payload, response }, rx)
    }
}
