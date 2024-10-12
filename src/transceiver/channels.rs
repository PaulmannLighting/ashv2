use crate::request::Request;
use log::error;
use std::io::{Error, ErrorKind};
use std::sync::mpsc::{Receiver, SyncSender, TryRecvError, TrySendError};
use std::task::Waker;

/// Communication channels of the transceiver.
#[derive(Debug)]
pub struct Channels {
    requests: Receiver<Request>,
    waker: Receiver<Waker>,
    callback: Option<SyncSender<Box<[u8]>>>,
    response: Option<SyncSender<Box<[u8]>>>,
}

impl Channels {
    /// Create a new set of communication channels.
    pub const fn new(
        requests: Receiver<Request>,
        waker: Receiver<Waker>,
        callback: Option<SyncSender<Box<[u8]>>>,
    ) -> Self {
        Self {
            requests,
            waker,
            callback,
            response: None,
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
        if let Some(response) = self.response.clone() {
            self.send_response(response, payload);
        } else if let Some(callback) = self.callback.clone() {
            self.send_callback(callback, payload);
        } else {
            error!("Neither response channel not callback channel are available. Discarding data.");
        }
    }

    /// Reset the response channel.
    pub fn reset(&mut self) {
        self.response.take();
    }

    /// Close the response channel.
    pub fn close(&mut self) {
        self.response.take();

        // Wake up all remaining wakers.
        while let Ok(waker) = self.waker.try_recv() {
            waker.wake();
        }
    }

    fn send_response(&mut self, response: SyncSender<Box<[u8]>>, payload: Box<[u8]>) {
        if let Err(error) = response.try_send(payload) {
            match error {
                TrySendError::Full(_) => {
                    error!("ASHv2: Response channel is congested. Dropping response frame.");
                }
                TrySendError::Disconnected(_) => {
                    self.response.take();
                    error!("ASHv2: Response channel has disconnected. Closing response channel.");
                }
            }
        }

        if let Ok(waker) = self.waker.try_recv() {
            waker.wake();
        }
    }

    fn send_callback(&mut self, callback: SyncSender<Box<[u8]>>, payload: Box<[u8]>) {
        if let Err(error) = callback.try_send(payload) {
            match error {
                TrySendError::Full(_) => {
                    error!("ASHv2: Callback channel is congested. Dropping callback frame.");
                }
                TrySendError::Disconnected(_) => {
                    self.callback.take();
                    error!(
                            "ASHv2: Callback channel has disconnected. Closing callback channel forever.",
                        );
                }
            }
        }
    }
}
