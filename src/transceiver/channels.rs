use crate::request::Request;
use log::error;
use std::io::{Error, ErrorKind};
use std::sync::mpsc::{Receiver, SyncSender, TryRecvError, TrySendError};

/// Communication channels of the transceiver.
#[derive(Debug)]
pub struct Channels {
    requests: Receiver<Request>,
    pub(super) response: Option<SyncSender<Box<[u8]>>>,
    callback: Option<SyncSender<Box<[u8]>>>,
}

impl Channels {
    /// Create a new set of communication channels.
    pub const fn new(requests: Receiver<Request>, callback: Option<SyncSender<Box<[u8]>>>) -> Self {
        Self {
            requests,
            response: None,
            callback,
        }
    }

    /// Receive a request from the host.
    pub fn receive(&mut self) -> std::io::Result<Option<Box<[u8]>>> {
        match self.requests.try_recv() {
            Ok(request) => {
                self.response.replace(request.response);
                Ok(Some(request.payload))
            }
            Err(error) => match error {
                TryRecvError::Empty => Ok(None),
                TryRecvError::Disconnected => Err(Error::new(
                    ErrorKind::BrokenPipe,
                    "ASHv2: Receiver channel disconnected.",
                )),
            },
        }
    }

    /// Respond to the host.
    pub fn respond(&mut self, payload: Box<[u8]>) {
        if let Some(response) = self.response.take() {
            if let Err(error) = response.try_send(payload) {
                match error {
                    TrySendError::Disconnected(_) => {
                        error!("ASHv2: Response channel has disconnected.");
                    }
                    TrySendError::Full(_) => error!("ASHv2: Response channel's buffer is full."),
                }
            }
        } else if let Some(callback) = &mut self.callback {
            if let Err(error) = callback.try_send(payload) {
                match error {
                    TrySendError::Disconnected(_) => {
                        self.callback.take();
                        error!(
                            "ASHv2: Callback channel has disconnected. Closing callback channel forever.",
                        );
                    }
                    TrySendError::Full(_) => error!("ASHv2: Callback channel's buffer is full."),
                }
            }
        } else {
            error!("Neither response channel not callback channel are available. Discarding data.");
        }
    }

    /// Reset the response channel.
    pub fn reset(&mut self) {
        self.response.take();
    }
}
