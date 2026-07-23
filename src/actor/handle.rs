use std::io;

use log::trace;
use tokio::sync::mpsc::Sender;
use tokio::sync::oneshot::channel;

use crate::Payload;
use crate::actor::message::Message;
use crate::hex_slice::HexSlice;

/// User-facing handle for sending payloads to the `ASHv2` actor.
///
/// Cloning a handle keeps the transmitter's message queue open. After every handle clone has been
/// dropped, the transmitter drains the queue and terminates. There is no explicit terminate
/// message.
#[derive(Clone, Debug)]
pub struct Handle {
    inner: Sender<Message>,
}

impl Handle {
    /// Send data to the `ASHv2` actor.
    ///
    /// # Errors
    ///
    /// Returns an error if the actor futures are no longer accepting messages or if the
    /// transmitter fails to write the payload.
    pub async fn send(&self, payload: Payload) -> io::Result<()> {
        let (response_tx, response_rx) = channel();

        trace!("Sending chunk: {:#04X}", HexSlice::new(&payload));
        self.inner
            .send(Message::Payload {
                payload: Box::new(payload),
                response_tx,
            })
            .await
            .map_err(io::Error::other)?;

        response_rx.await.map_err(io::Error::other)?
    }
}

impl From<Sender<Message>> for Handle {
    fn from(inner: Sender<Message>) -> Self {
        Self { inner }
    }
}
