use std::io;

use log::trace;
use tokio::sync::mpsc::Sender;
use tokio::sync::mpsc::error::SendError;
use tokio::sync::oneshot::{Receiver, channel};

use crate::Payload;
use crate::actor::message::Message;
use crate::utils::HexSlice;

type Response = Receiver<io::Result<()>>;
type Error = SendError<Message>;

/// Default proxy type.
pub type DefaultProxy = Sender<Message>;

/// `ASHv2` actor proxy.
pub trait Proxy {
    /// Send data to the `ASHv2` actor.
    ///
    /// # Errors
    ///
    /// Returns an [`Error`] if sending the message fails.
    fn send(&self, payload: Payload) -> impl Future<Output = Result<Response, Error>> + Send;
}

impl Proxy for Sender<Message> {
    async fn send(&self, payload: Payload) -> Result<Response, Error> {
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
