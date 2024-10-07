use crate::request::Request;
use log::error;
use std::io::{Error, ErrorKind};
use std::sync::mpsc::{Receiver, SendError, Sender, TryRecvError};

/// Communication channels of the transceiver.
#[derive(Debug)]
pub struct Channels {
    requests: Receiver<Request>,
    pub(super) response: Option<Sender<Box<[u8]>>>,
    callback: Option<Sender<Box<[u8]>>>,
}

impl Channels {
    /// Create a new set of communication channels.
    pub const fn new(requests: Receiver<Request>, callback: Option<Sender<Box<[u8]>>>) -> Self {
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
            if let Err(error) = response.send(payload) {
                match error {
                    SendError(_) => {
                        error!("ASHv2: Response channel has disconnected.");
                    }
                }
            }
        } else if let Some(callback) = &mut self.callback {
            if let Err(error) = callback.send(payload) {
                match error {
                    SendError(_) => {
                        self.callback.take();
                        error!(
                            "ASHv2: Callback channel has disconnected. Closing callback channel forever.",
                        );
                    }
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
