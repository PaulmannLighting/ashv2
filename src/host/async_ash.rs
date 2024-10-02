//! Asynchronous host controller for the `ASHv2` protocol.

mod async_request;

use crate::request::Request;
use async_request::AsyncRequest;
use std::future::Future;
use std::sync::mpsc::Sender;

/// A host controller to communicate with an NCP via the `ASHv2` protocol.
pub trait AsyncAsh {
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

impl AsyncAsh for Sender<Request> {
    async fn communicate(&self, payload: &[u8]) -> <AsyncRequest as Future>::Output {
        AsyncRequest::new(self.clone(), payload).await
    }
}
