use std::io::{ErrorKind, Result};

use bytes::Bytes;
use tokio::sync::mpsc::Sender;
use tokio::sync::oneshot::channel;

use super::message::Message;

pub struct Writer(pub(crate) Sender<Message>);

impl Writer {
    pub async fn write(&self, buffer: Bytes) -> Result<()> {
        let (response, rx) = channel();

        self.0
            .send(Message::Write { buffer, response })
            .await
            .map_err(|_| ErrorKind::BrokenPipe)?;

        rx.await.map_err(|_| ErrorKind::BrokenPipe)?
    }
}
