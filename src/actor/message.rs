use std::io;

use tokio::sync::oneshot::Sender;

use crate::Payload;

/// Application request to send one `DATA` frame and await acknowledgement.
#[derive(Debug)]
pub struct Message {
    /// Payload to transmit.
    pub payload: Box<Payload>,
    /// Completion channel for acknowledgement or terminal failure.
    pub response_tx: Sender<io::Result<()>>,
}
