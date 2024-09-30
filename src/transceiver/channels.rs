use crate::request::Request;
use log::{error, warn};
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
    pub fn receive(&mut self) -> Result<Box<[u8]>, TryRecvError> {
        match self.requests.try_recv() {
            Ok(request) => {
                self.response.replace(request.response);
                Ok(request.payload)
            }
            Err(error) => Err(error),
        }
    }

    pub fn response(
        &self,
        payload: std::io::Result<Box<[u8]>>,
    ) -> Result<(), SendError<std::io::Result<Box<[u8]>>>> {
        self.response.as_ref().map_or_else(
            || {
                warn!("No response channel set. Discarding response.");
                Ok(())
            },
            |response| {
                response
                    .send(payload)
                    .inspect_err(|error| error!("ASHv2 failed to send response: {error}"))
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
