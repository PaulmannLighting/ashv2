use std::io;
use std::io::ErrorKind;

use tokio::sync::mpsc::Sender;
use tokio::sync::oneshot::channel;

use crate::Payload;
use crate::actor::message::Message;

/// User-facing handle for sending payloads to the `ASHv2` actor.
///
/// Cloning a handle keeps the transmitter's message queue open. After every handle clone has been
/// dropped, the transmitter finishes queued and in-flight transmissions before terminating.
/// A link failure ends the session and fails queued and in-flight requests.
#[derive(Clone, Debug)]
pub struct Handle {
    inner: Sender<Message>,
}

impl Handle {
    /// Send a payload and wait for the NCP's ASH acknowledgement.
    ///
    /// Requests wait in a bounded queue while the transmit window is full. The acknowledgement
    /// confirms receipt of the ASH frame. Any EZSP response arrives through the inbound payload
    /// channel.
    ///
    /// Dropping the future stops waiting. An already admitted frame can still be sent or retried.
    ///
    /// # Errors
    ///
    /// Returns an error if the session ends before acknowledgement. Retry exhaustion returns
    /// [`ErrorKind::TimedOut`] for in-flight and queued requests. A closed queue returns
    /// [`ErrorKind::BrokenPipe`] to requests still waiting for admission.
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
