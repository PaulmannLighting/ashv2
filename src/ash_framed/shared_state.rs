use std::sync::mpsc::Receiver;
use std::task::Waker;

#[derive(Debug, Default)]
pub struct SharedState {
    pub(super) receiver: Option<Receiver<Box<[u8]>>>,
    pub(super) buffer: Vec<u8>,
    pub(super) result: Option<std::io::Result<Box<[u8]>>>,
    pub(super) waker: Option<Waker>,
}

impl SharedState {
    pub fn reset(&mut self) {
        self.receiver = None;
        self.buffer.clear();
        self.result = None;
        self.waker = None;
    }
}
