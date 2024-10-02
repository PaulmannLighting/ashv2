//! Synchronous host controller for the `ASHv2` protocol.

use crate::request::Request;
use std::io::ErrorKind;
use std::sync::mpsc::Sender;

/// A host controller to communicate with an NCP via the `ASHv2` protocol.
pub trait CommunicateSync {
    /// Communicate with the NCP, returning `Box<[u8]>`.
    ///
    /// # Errors
    ///
    /// Returns [`std::io::Error`] if the transactions fails.
    fn communicate(&self, payload: &[u8]) -> std::io::Result<Box<[u8]>>;
}

impl CommunicateSync for Sender<Request> {
    fn communicate(&self, payload: &[u8]) -> std::io::Result<Box<[u8]>> {
        let (request, response) = Request::new(payload.into());
        self.send(request).map_err(|_| {
            std::io::Error::new(ErrorKind::BrokenPipe, "ASHv2 failed to send request.")
        })?;
        response.recv().map_err(|_| {
            std::io::Error::new(
                ErrorKind::BrokenPipe,
                "ASHv2 response channel disconnected.",
            )
        })?
    }
}
