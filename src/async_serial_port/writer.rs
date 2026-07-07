use std::io::{ErrorKind, Result};

use tokio::sync::mpsc::Sender;
use tokio::sync::oneshot::channel;

use super::message::Message;

pub struct Writer(pub(crate) Sender<Message>);

impl Writer {
    pub async fn write(&self, buf: &[u8]) -> Result<()> {
        let (response, rx) = channel();

        self.0
            .send(Message::Write {
                buffer: buf.into(),
                response,
            })
            .await
            .map_err(|_| ErrorKind::BrokenPipe)?;

        rx.await.map_err(|_| ErrorKind::BrokenPipe)?
    }
}
