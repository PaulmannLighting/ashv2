use std::sync::mpsc::SyncSender;

/// A request sent by [`AshFramed`](crate::AshFramed) to the [`Transceiver`](crate::Transceiver).
#[derive(Debug)]
pub struct Request {
    pub(crate) payload: Box<[u8]>,
    pub(crate) response: SyncSender<Box<[u8]>>,
}

impl Request {
    #[must_use]
    pub(crate) const fn new(payload: Box<[u8]>, response: SyncSender<Box<[u8]>>) -> Self {
        Self { payload, response }
    }
}
