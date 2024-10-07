use std::sync::mpsc::{channel, Receiver, Sender};

/// An incoming request.
#[derive(Debug)]
pub struct Request {
    pub(crate) payload: Box<[u8]>,
    pub(crate) response: Sender<Box<[u8]>>,
}

impl Request {
    #[must_use]
    pub(crate) fn new(payload: Box<[u8]>) -> (Self, Receiver<Box<[u8]>>) {
        let (response, rx) = channel();
        (Self { payload, response }, rx)
    }
}
