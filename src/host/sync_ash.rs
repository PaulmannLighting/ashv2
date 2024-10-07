//! Synchronous host controller for the `ASHv2` protocol.

use crate::host::Host;
use crate::request::Request;
use std::io::ErrorKind;

/// A trait to synchronously (blocking) communicate with an NCP via the `ASHv2` protocol.
pub trait SyncAsh {
    /// Communicate with the NCP, returning `Box<[u8]>`.
    ///
    /// # Errors
    ///
    /// Returns [`std::io::Error`] if the transactions fails.
    fn communicate(&self, payload: &[u8]) -> std::io::Result<Box<[u8]>>;
}

impl<T> SyncAsh for T
where
    T: Host,
{
    fn communicate(&self, payload: &[u8]) -> std::io::Result<Box<[u8]>> {
        let (request, response) = Request::new(payload.into());
        self.send(request).map_err(|_| {
            std::io::Error::new(ErrorKind::BrokenPipe, "ASHv2: Failed to send request.")
        })?;
        response.recv().map_err(|_| {
            std::io::Error::new(
                ErrorKind::BrokenPipe,
                "ASHv2: Response channel disconnected.",
            )
        })?
    }
}
