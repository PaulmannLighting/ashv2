use crate::request::Request;
use log::{error, warn};
use std::io::{Error, ErrorKind};
use std::sync::mpsc::{Receiver, SyncSender, TryRecvError, TrySendError};

/// Communication channels of the transceiver.
#[derive(Debug)]
pub struct Channels {
    requests: Receiver<Request>,
    response: Option<SyncSender<std::io::Result<Box<[u8]>>>>,
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
                    "ASHv2 receiver channel disconnected",
                )),
            },
        }
    }

    /// Respond to the host.
    pub fn respond(&mut self, payload: std::io::Result<Box<[u8]>>) -> std::io::Result<()> {
        let Some(response) = self.response.take() else {
            error!("No response channel set. Discarding response.");
            return Ok(());
        };

        if let Err(error) = response.try_send(payload) {
            match error {
                TrySendError::Disconnected(_) => Err(Error::new(
                    ErrorKind::BrokenPipe,
                    "Response channel has disconnected.",
                )),
                TrySendError::Full(_) => Err(Error::new(
                    ErrorKind::OutOfMemory,
                    "Response channel's buffer is full.",
                )),
            }
        } else {
            Ok(())
        }
    }

    /// Send a callback via the callback channel.
    pub fn callback(&mut self, payload: Box<[u8]>) -> std::io::Result<()> {
        let Some(callback) = self.callback.as_ref() else {
            warn!("No callback set. Discarding response.");
            return Ok(());
        };

        if let Err(error) = callback.try_send(payload) {
            match error {
                TrySendError::Disconnected(_) => {
                    self.callback.take();
                    Err(Error::new(
                        ErrorKind::BrokenPipe,
                        "Callback channel has disconnected. Closing callback channel forever.",
                    ))
                }
                TrySendError::Full(_) => Err(Error::new(
                    ErrorKind::OutOfMemory,
                    "Callback channel's buffer is full.",
                )),
            }
        } else {
            Ok(())
        }
    }

    /// Reset the response channel.
    pub fn reset(&mut self) {
        self.response.take();
    }
}
