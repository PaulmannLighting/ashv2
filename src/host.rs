use std::future::Future;
use std::sync::mpsc::{channel, Sender};
use std::thread::{spawn, JoinHandle};

use log::error;
use serialport::TTYPort;

use crate::error::Error;
use crate::request::Request;
use crate::response::Response;
use crate::transceiver::Transceiver;

mod listener;
mod transmitter;

/// A host controller to communicate with an NCP via the `ASHv2` protocol.
#[derive(Debug)]
pub struct Host {
    command: Sender<Request>,
    transceiver: Option<JoinHandle<()>>,
}

impl Host {
    /// Creates and starts the host.
    #[must_use]
    pub fn new(serial_port: TTYPort, callback: Option<Sender<Box<[u8]>>>) -> Self {
        let (command, requests) = channel();

        Self {
            command,
            transceiver: Some(spawn(move || {
                Transceiver::new(serial_port, requests, callback).run();
            })),
        }
    }

    /// Communicate with the NCP, returning `Box<[u8]>`.
    ///
    /// # Errors
    ///
    /// Returns [`std::io::Error`] if the transactions fails.
    pub async fn communicate(&self, payload: &[u8]) -> <Response as Future>::Output {
        let (request, response) = Request::new(payload.into());
        match self.command.send(request) {
            Ok(()) => Response::new(response).await,
            Err(_) => Response::failed().await,
        }
    }
}

impl Drop for Host {
    fn drop(&mut self) {
        // TODO: Signal thread to shut down.

        if let Some(thread) = self.transceiver.take() {
            thread.join().unwrap_or_else(|_| {
                error!("Failed to join transceiver thread.");
            });
        }
    }
}
