mod buffers;
mod channels;
mod constants;
mod implementations;
mod state;
mod transmission;

use crate::protocol::AshChunks;
use crate::status::Status;
use crate::Request;
use buffers::Buffers;
use channels::Channels;
use serialport::SerialPort;
use state::State;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering::Relaxed;
use std::sync::mpsc::{Receiver, Sender};
use std::sync::Arc;

/// `ASHv2` transceiver.
///
/// The transceiver is responsible for handling the communication between the host and the NCP.
/// It is supposed to be run in a separate thread.
///
/// The [`AsyncAsh`](crate::AsyncAsh) and [`SyncAsh`](crate::SyncAsh) traits can be used to
/// provide the sender of the channel wih a method to communicate with the NCP via the transceiver.
///
#[derive(Debug)]
pub struct Transceiver<T>
where
    T: SerialPort,
{
    serial_port: T,
    channels: Channels,
    buffers: Buffers,
    state: State,
}

impl<T> Transceiver<T>
where
    T: SerialPort,
{
    /// Create a new transceiver.
    ///
    /// # Parameters
    ///
    /// - `serial_port`: The serial port to communicate with the NCP.
    /// - `requests`: The channel to receive requests from the host.
    /// - `callback`: An optional channel to send callbacks from the NCP to.
    ///
    /// If no callback channel is provided, the transceiver will
    /// silently discard any callbacks actively sent from the NCP.
    #[must_use]
    pub fn new(
        serial_port: T,
        requests: Receiver<Request>,
        callback: Option<Sender<Box<[u8]>>>,
    ) -> Self {
        Self {
            serial_port,
            channels: Channels::new(requests, callback),
            buffers: Buffers::default(),
            state: State::new(),
        }
    }

    /// Run the transceiver.
    ///
    /// This should be called in a separate thread.
    #[allow(clippy::needless_pass_by_value)]
    pub fn run(mut self, running: Arc<AtomicBool>) {
        while running.load(Relaxed) {
            if let Err(error) = self.main() {
                self.handle_io_error(&error);
            }
        }
    }

    /// Main loop of the transceiver.
    ///
    /// This method checks whether the transceiver is connected and establishes a connection if not.
    /// Otherwise, it will communicate with the NCP via the `ASHv2` protocol.
    fn main(&mut self) -> std::io::Result<()> {
        match self.state.status {
            Status::Disconnected | Status::Failed => Ok(self.connect()?),
            Status::Connected => self.communicate(),
        }
    }

    /// Communicate with the NCP.
    ///
    /// If there is an incoming transaction, handle it.
    /// Otherwise, handle callbacks.
    fn communicate(&mut self) -> std::io::Result<()> {
        if let Some(bytes) = self.channels.receive()? {
            self.transaction(bytes.ash_chunks()?)
        } else {
            self.handle_callbacks()
        }
    }
}
