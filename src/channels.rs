use crate::request::Request;
use log::{error, warn};
use std::io::ErrorKind;
use std::sync::mpsc::{Receiver, SendError, Sender, TryRecvError};

/// Communication channels of the transceiver.
#[derive(Debug)]
pub struct Channels {
    requests: Receiver<Request>,
    response: Option<Sender<std::io::Result<Box<[u8]>>>>,
    callback: Option<Sender<Box<[u8]>>>,
}

impl Channels {
    pub const fn new(requests: Receiver<Request>, callback: Option<Sender<Box<[u8]>>>) -> Self {
        Self {
            requests,
            response: None,
            callback,
        }
    }
    pub fn receive(&mut self) -> std::io::Result<Option<Box<[u8]>>> {
        match self.requests.try_recv() {
            Ok(request) => {
                self.response.replace(request.response);
                Ok(Some(request.payload))
            }
            Err(error) => match error {
                TryRecvError::Empty => Ok(None),
                TryRecvError::Disconnected => Err(std::io::Error::new(
                    ErrorKind::BrokenPipe,
                    "ASHv2 receiver channel disconnected",
                )),
            },
        }
    }

    pub fn response(&self, payload: std::io::Result<Box<[u8]>>) -> std::io::Result<()> {
        self.response.as_ref().map_or_else(
            || {
                warn!("No response channel set. Discarding response.");
                Ok(())
            },
            |response| {
                response
                    .send(payload)
                    .inspect_err(|error| error!("ASHv2 failed to send response: {error}"))
                    .map_err(|_| {
                        std::io::Error::new(ErrorKind::BrokenPipe, "ASHv2 failed to send reponse")
                    })
            },
        )
    }

    pub fn callback(&self, payload: Box<[u8]>) -> Result<(), SendError<Box<[u8]>>> {
        self.callback.as_ref().map_or_else(
            || {
                warn!("No callback set. Discarding response.");
                Ok(())
            },
            |callback| callback.send(payload),
        )
    }
}