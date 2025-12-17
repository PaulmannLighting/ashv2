use tokio::io;
use tokio::sync::mpsc::Sender;
use tokio::sync::mpsc::error::SendError;
use tokio::sync::oneshot::{Receiver, channel};

use crate::actor::message::Message;

type TransmitResult = Result<Receiver<io::Result<()>>, SendError<Message>>;

/// Trait to transmit data through a channel via an `ASHv2` transmitter.
pub trait Transmit {
    /// Transmit bytes.
    ///
    /// # Errors
    ///
    /// Returns a [`SendError`] if the message could not be sent through the channel.
    fn transmit<T>(&self, data: T) -> impl Future<Output = TransmitResult>
    where
        T: AsRef<[u8]>;
}

impl Transmit for Sender<Message> {
    async fn transmit<T>(&self, data: T) -> TransmitResult
    where
        T: AsRef<[u8]>,
    {
        let (response_tx, response_rx) = channel();

        self.send(Message::Payload {
            payload: Box::from(data.as_ref()),
            response: response_tx,
        })
        .await
        .map(|()| response_rx)
    }
}
