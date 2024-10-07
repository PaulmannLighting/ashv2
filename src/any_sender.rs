use crate::request::Request;
use std::sync::mpsc::{SendError, Sender, SyncSender};

/// A trait to identify types that can be used as `ASHv2` hosts.
pub trait AnySender {
    /// Send a request to the transceiver.
    ///
    /// # Errors
    ///
    /// Returns a [`SendError`] if the request could not be sent.
    fn send(&self, request: Request) -> Result<(), SendError<Request>>;
}

impl AnySender for Sender<Request> {
    fn send(&self, request: Request) -> Result<(), SendError<Request>> {
        Self::send(self, request)
    }
}

impl AnySender for SyncSender<Request> {
    fn send(&self, request: Request) -> Result<(), SendError<Request>> {
        Self::send(self, request)
    }
}
