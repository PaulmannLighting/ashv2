use std::sync::mpsc::SyncSender;

use crate::Payload;

/// A request sent by [`AshFramed`](crate::AshFramed) to the [`Transceiver`](crate::Transceiver).
#[derive(Debug)]
pub struct Request {
    pub(crate) payload: Box<[u8]>,
    pub(crate) response: SyncSender<Payload>,
}

impl Request {
    /// Creates a new request.
    #[must_use]
    pub const fn new(payload: Box<[u8]>, response: SyncSender<Payload>) -> Self {
        Self { payload, response }
    }
}
