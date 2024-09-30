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
    ///
    /// # Errors
    /// Returns an [`Error`] if the host could not be started.
    pub fn spawn(serial_port: TTYPort, callback: Option<Sender<Box<[u8]>>>) -> Result<Self, Error> {
        let (command, requests) = channel();
        let transceiver = Transceiver::new(serial_port, requests, callback);

        Ok(Self {
            command,
            transceiver: Some(spawn(move || transceiver.run())),
        })
    }

    /// Communicate with the NCP, returning `Box<[u8]>`.
    ///
    /// # Errors
    ///
    /// Returns [`std::io::Error`] if the transactions fails.
    pub async fn communicate<T>(&self, payload: &[u8]) -> <Response as Future>::Output {
        let (request, response) = Request::new(payload.into());
        self.command.send(request).expect("Failed to send request.");
        Response::new(response).await
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
