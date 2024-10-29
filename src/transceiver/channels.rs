use std::io::{Error, ErrorKind};

use log::error;
use tokio::sync::mpsc::error::{TryRecvError, TrySendError};
use tokio::sync::mpsc::{Receiver, Sender};

use crate::Payload;

/// Communication channels of the transceiver.
#[derive(Debug)]
pub struct Channels {
    requests: Receiver<Payload>,
    response: Sender<std::io::Result<Payload>>,
}

impl Channels {
    /// Create a new set of communication channels.
    pub const fn new(
        requests: Receiver<Payload>,
        response: Sender<std::io::Result<Payload>>,
    ) -> Self {
        Self { requests, response }
    }

    /// Receive a request from the host.
    pub fn receive(&mut self) -> std::io::Result<Option<Payload>> {
        match self.requests.try_recv() {
            Ok(request) => Ok(Some(request)),
            Err(error) => match error {
                TryRecvError::Empty => Ok(None),
                TryRecvError::Disconnected => Err(Error::new(
                    ErrorKind::BrokenPipe,
                    "Receiver channel disconnected.",
                )),
            },
        }
    }

    /// Respond to the host.
    pub fn respond(&self, result: std::io::Result<Payload>) {
        if let Err(error) = self.response.try_send(result) {
            match error {
                TrySendError::Full(_) => {
                    error!("Response channel is congested. Dropping response frame.");
                }
                TrySendError::Closed(_) => {
                    // TODO: Maybe panic here?
                    error!("Response channel has disconnected. Closing response channel.");
                }
            }
        }
    }
}
