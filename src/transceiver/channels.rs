//! Communication channels for the transceiver.

use std::io::{self, Error, ErrorKind};

use log::{error, trace};
use tokio::sync::mpsc::error::{TryRecvError, TrySendError};
use tokio::sync::mpsc::{Receiver, Sender};

use crate::Payload;

/// Communication channels of the transceiver.
#[derive(Debug)]
pub struct Channels {
    requests: Receiver<Payload>,
    response: Sender<io::Result<Payload>>,
}

impl Channels {
    /// Create a new set of communication channels.
    pub const fn new(requests: Receiver<Payload>, response: Sender<io::Result<Payload>>) -> Self {
        Self { requests, response }
    }

    /// Receive a request from the host.
    pub fn receive(&mut self) -> io::Result<Option<Payload>> {
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
    pub fn respond(&self, result: io::Result<Payload>) {
        if let Err(error) = self.response.try_send(result) {
            match error {
                TrySendError::Full(payload) => {
                    error!("Response channel is congested. Dropping response frame.");
                    trace!("Response frame was: {payload:?}");
                }
                TrySendError::Closed(payload) => {
                    panic!("Response channel has disconnected. Response was: {payload:?}");
                }
            }
        }
    }
}
