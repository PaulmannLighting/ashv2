use log::trace;
use tokio::sync::mpsc::Sender;
use tokio::sync::oneshot::channel;

pub use self::error::Error;
use crate::actor::message::Message;
use crate::{HexSlice, Payload};

mod error;

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

    /// Send data through the `ASHv2` actor.
    ///
    /// # Errors
    ///
    /// Returns an [`Error`] if sending the message fails, receiving the response fails or if there was an I/O error.
    pub async fn communicate(&self, payload: Payload) -> Result<(), Error> {
        let (response_tx, response_rx) = channel();

        trace!("Sending chunk: {:#04X}", HexSlice::new(&payload));
        self.sender
            .send(Message::Payload {
                payload: Box::new(payload),
                response: response_tx,
            })
            .await?;
        trace!("Awaiting response from back-channel...");
        let result = response_rx.await?;
        trace!("Resolving result from back-channel...");
        result?;
        Ok(())
    }
}
