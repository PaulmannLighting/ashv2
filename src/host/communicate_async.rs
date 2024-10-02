//! Asynchronous host controller for the `ASHv2` protocol.

mod response;

use crate::host::Host;
use crate::request::Request;
use response::Response;
use std::future::Future;

/// A host controller to communicate with an NCP via the `ASHv2` protocol.
pub trait CommunicateAsync {
    /// Communicate with the NCP, returning `Box<[u8]>`.
    ///
    /// # Errors
    ///
    /// Returns [`std::io::Error`] if the transactions fails.
    fn communicate(
        &self,
        payload: &[u8],
    ) -> impl Future<Output = std::io::Result<Box<[u8]>>> + Send;
}

impl CommunicateAsync for Host {
    async fn communicate(&self, payload: &[u8]) -> <Response as Future>::Output {
        let (request, response) = Request::new(payload.into());
        match self.command.send(request) {
            Ok(()) => Response::new(response).await,
            Err(_) => Response::failed().await,
        }
    }
}
