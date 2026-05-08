use std::io;
use std::io::Error;

use log::trace;
use tokio::sync::mpsc::Sender;
use tokio::sync::oneshot::{Receiver, channel};

use crate::Payload;
use crate::actor::message::Message;
use crate::utils::HexSlice;

type Response = Receiver<io::Result<()>>;

/// `ASHv2` actor proxy.
#[derive(Clone, Debug)]
pub struct Proxy {
    inner: Sender<Message>,
}

impl Proxy {
    /// Send data to the `ASHv2` actor.
    ///
    /// # Errors
    ///
    /// Returns an [`SendError<Message>`] if sending the message fails.
    pub async fn send(&self, payload: Payload) -> io::Result<()> {
        let (response_tx, response_rx) = channel();

        trace!("Sending chunk: {:#04X}", HexSlice::new(&payload));
        self.inner
            .send(Message::Payload {
                payload: Box::new(payload),
                response_tx,
            })
            .await
            .map_err(|_| Error::other("Failed to send payload to actor"))?;

        response_rx
            .await
            .map_err(|_| Error::other("Failed to receive payload to actor"))?
    }
}

impl From<Sender<Message>> for Proxy {
    fn from(inner: Sender<Message>) -> Self {
        Self { inner }
    }
}
