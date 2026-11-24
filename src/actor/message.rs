use std::io;

use tokio::sync::oneshot::Sender;

use crate::Payload;

/// Application request to write one `DATA` frame.
#[derive(Debug)]
pub struct Message {
    pub payload: Box<Payload>,
    pub response_tx: Sender<io::Result<()>>,
}
