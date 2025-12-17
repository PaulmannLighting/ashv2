use tokio::sync::mpsc::Sender;
use tokio::sync::oneshot::channel;

pub use self::error::Error;
use crate::Payload;
use crate::actor::message::Message;

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

        self.sender
            .send(Message::Payload {
                payload: Box::new(payload),
                response: response_tx,
            })
            .await?;
        response_rx.await??;
        Ok(())
    }
}
