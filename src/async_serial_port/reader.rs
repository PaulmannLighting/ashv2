use std::io::{ErrorKind, Result};

use bytes::BytesMut;
use tokio::sync::mpsc::Sender;
use tokio::sync::oneshot::channel;

use super::message::Message;

pub struct Reader(pub(crate) Sender<Message>);

impl Reader {
    pub async fn read(&self) -> Result<BytesMut> {
        let buffer = BytesMut::new();
        let (response, rx) = channel();

        self.0
            .send(Message::Read { buffer, response })
            .await
            .map_err(|_| ErrorKind::BrokenPipe)?;

        rx.await.map_err(|_| ErrorKind::BrokenPipe)?
    }
}
