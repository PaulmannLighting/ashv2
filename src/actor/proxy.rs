use std::io;

use log::trace;
use tokio::sync::mpsc::Sender;
use tokio::sync::mpsc::error::SendError;
use tokio::sync::oneshot::{Receiver, channel};

use crate::actor::message::Message;
use crate::{HexSlice, Payload};

type Response = Receiver<io::Result<()>>;
type Error = SendError<Message>;

/// `ASHv2` actor proxy.
#[derive(Clone, Debug)]
pub struct Proxy {
    sender: Sender<Message>,
}

impl Proxy {
    /// Create a new `ASHv2` actor proxy.
    #[must_use]
    pub(crate) const fn new(sender: Sender<Message>) -> Self {
        Self { sender }
    }

    /// Send data to the `ASHv2` actor.
    ///
    /// # Errors
    ///
    /// Returns an [`Error`] if sending the message fails.
    pub async fn send(&self, payload: Payload) -> Result<Response, Error> {
        let (response_tx, response_rx) = channel();

        trace!("Sending chunk: {:#04X}", HexSlice::new(&payload));
        self.sender
            .send(Message::Payload {
                payload: Box::new(payload),
                response_tx,
            })
            .await?;

        Ok(response_rx)
    }
}
