use std::io;
use std::io::ErrorKind;

use tokio::sync::mpsc::Sender;
use tokio::sync::oneshot::channel;

use crate::Payload;
use crate::actor::message::Message;

/// User-facing handle for sending payloads to the `ASHv2` actor.
///
/// Cloning a handle keeps the transmitter's message queue open. After every handle clone has been
/// dropped, the transmitter drains the queue and terminates. A link failure ends the session
/// and fails queued requests.
#[derive(Clone, Debug)]
pub struct Handle {
    inner: Sender<Message>,
}

impl Handle {
    /// Send data to the `ASHv2` actor and wait for the local transport write.
    ///
    /// # Errors
    ///
    /// Returns an error if the session ends before the payload can be written or if the
    /// transport write fails.
    pub async fn send(&self, payload: Payload) -> io::Result<()> {
        let (response_tx, response_rx) = channel();

        self.inner
            .send(Message {
                payload: Box::new(payload),
                response_tx,
            })
            .await
            .map_err(|_| ErrorKind::BrokenPipe)?;

        response_rx.await.map_err(|_| ErrorKind::BrokenPipe)?
    }
}

impl From<Sender<Message>> for Handle {
    fn from(inner: Sender<Message>) -> Self {
        Self { inner }
    }
}
