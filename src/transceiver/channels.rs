use std::io::{Error, ErrorKind};

use log::error;
use tokio::sync::mpsc::error::{TryRecvError, TrySendError};
use tokio::sync::mpsc::{Receiver, Sender};

use crate::Payload;

/// Communication channels of the transceiver.
#[derive(Debug)]
pub struct Channels {
    requests: Receiver<Box<[u8]>>,
    response: Sender<Payload>,
}

impl Channels {
    /// Create a new set of communication channels.
    pub const fn new(requests: Receiver<Box<[u8]>>, response: Sender<Payload>) -> Self {
        Self { requests, response }
    }

    /// Receive a request from the host.
    pub fn receive(&mut self) -> std::io::Result<Option<Box<[u8]>>> {
        match self.requests.try_recv() {
            Ok(payload) => Ok(Some(payload)),
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
    pub fn respond(&self, payload: Payload) {
        if let Err(error) = self.response.try_send(payload) {
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
