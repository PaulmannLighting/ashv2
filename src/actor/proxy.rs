use std::io;

use log::trace;
use tokio::sync::mpsc::Sender;
use tokio::sync::mpsc::error::SendError;
use tokio::sync::oneshot::{Receiver, channel};

use crate::actor::message::Message;
use crate::{HexSlice, Payload};

/// `ASHv2` actor proxy.
pub trait Proxy {
    /// Response type for sending data to the `ASHv2` actor.
    type Response;

    /// Error type for sending data to the `ASHv2` actor.
    type Error;

    /// Send data to the `ASHv2` actor.
    ///
    /// # Errors
    ///
    /// Returns an [`Error`] if sending the message fails.
    fn send(
        &self,
        payload: Payload,
    ) -> impl Future<Output = Result<Self::Response, Self::Error>> + Send;
}

impl Proxy for Sender<Message> {
    type Response = Receiver<io::Result<()>>;
    type Error = SendError<Message>;

    async fn send(&self, payload: Payload) -> Result<Self::Response, Self::Error> {
        let (response_tx, response_rx) = channel();

        trace!("Sending chunk: {:#04X}", HexSlice::new(&payload));
        self.send(Message::Payload {
            payload: Box::new(payload),
            response_tx,
        })
        .await?;

        Ok(response_rx)
    }
}
