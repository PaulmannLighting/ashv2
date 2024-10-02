//! Synchronous host controller for the `ASHv2` protocol.

use crate::request::Request;
use crate::Host;
use std::io::ErrorKind;

/// A host controller to communicate with an NCP via the `ASHv2` protocol.
pub trait CommunicateSync {
    /// Communicate with the NCP, returning `Box<[u8]>`.
    ///
    /// # Errors
    ///
    /// Returns [`std::io::Error`] if the transactions fails.
    fn communicate(&self, payload: &[u8]) -> std::io::Result<Box<[u8]>>;
}

impl CommunicateSync for Host {
    fn communicate(&self, payload: &[u8]) -> std::io::Result<Box<[u8]>> {
        let (request, response) = Request::new(payload.into());
        self.command.send(request).map_err(|_| {
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
