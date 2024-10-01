use std::future::Future;
use std::sync::mpsc::Sender;

use crate::request::Request;
use crate::response::Response;

/// A host controller to communicate with an NCP via the `ASHv2` protocol.
#[derive(Debug)]
pub struct Host {
    command: Sender<Request>,
}

impl Host {
    /// Creates and starts the host.
    #[must_use]
    pub const fn new(command: Sender<Request>) -> Self {
        Self { command }
    }

    /// Communicate with the NCP, returning `Box<[u8]>`.
    ///
    /// # Errors
    ///
    /// Returns [`std::io::Error`] if the transactions fails.
    pub async fn communicate(&self, payload: &[u8]) -> <Response as Future>::Output {
        let (request, response) = Request::new(payload.into());
        match self.command.send(request) {
            Ok(()) => Response::new(response).await,
            Err(_) => Response::failed().await,
        }
    }
}

impl From<Sender<Request>> for Host {
    fn from(command: Sender<Request>) -> Self {
        Self::new(command)
    }
}
