use std::sync::mpsc::Receiver;

#[derive(Debug, Default)]
pub struct SharedState {
    pub(super) sending: bool,
    pub(super) sent_bytes: Option<std::io::Result<usize>>,
    pub(super) receiver: Option<Receiver<Box<[u8]>>>,
    pub(super) buffer: Vec<u8>,
    pub(super) result: Option<std::io::Result<Box<[u8]>>>,
}

impl SharedState {
    pub fn reset(&mut self) {
        self.sending = false;
        self.sent_bytes = None;
        self.receiver = None;
        self.buffer.clear();
        self.result = None;
    }
}
