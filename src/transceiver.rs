mod buffers;
mod implementations;
mod state;

use crate::channels::Channels;
use crate::protocol::AshChunks;
use crate::request::Request;
use crate::status::Status;
use crate::transceiver::buffers::Buffers;
use crate::transceiver::state::State;
use log::error;
use serialport::TTYPort;
use std::sync::mpsc::{Receiver, Sender};

/// ASHv2 transceiver.
///
/// The transceiver is responsible for handling the communication between the host and the NCP.
/// It is supposed to be run in a separate thread.
/// A [`Host`](crate::Host) can be used to communicate with the NCP via the transceiver.
///
/// # Usage
///
/// ```
/// use std::sync::mpsc::channel;
/// use std::thread::spawn;
/// use serialport::FlowControl;
/// use tokio::task::futures;
/// use ashv2::{open, BaudRate, Host, Transceiver};
/// use ::futures::executor;
///
/// match open("/dev/ttyUSB0", BaudRate::RstCts, FlowControl::Software) {
///     Ok(serial_port) => {let (sender, receiver) = channel();
///         let transceiver = Transceiver::new(serial_port, receiver, None);
///         let _thread_handle = spawn(move || transceiver.run());
///         let host = Host::from(sender);
///
///         let version_command = &[0x00, 0x01, 0x02, 0x03];
///         let future = host.communicate(version_command);
///
///         match executor::block_on(future) {
///             Ok(response) => println!("{response:?}"),
///             Err(error) => eprintln!("{error}"),
///         }
///     },
///     Err(error) => eprintln!("{error}"),
/// }
///
/// ```
#[derive(Debug)]
pub struct Transceiver {
    serial_port: TTYPort,
    channels: Channels,
    buffers: Buffers,
    state: State,
}

impl Transceiver {
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
        serial_port: TTYPort,
        requests: Receiver<Request>,
        callback: Option<Sender<Box<[u8]>>>,
    ) -> Self {
        Self {
            serial_port,
            channels: Channels::new(requests, callback),
            buffers: Buffers::default(),
            state: State::default(),
        }
    }

    /// Run the transceiver.
    ///
    /// This should be called in a separate thread.
    pub fn run(mut self) {
        loop {
            if let Err(error) = self.main() {
                error!("I/O error: {error}");
                self.reset(Status::Failed);
            }
        }
    }

    fn main(&mut self) -> std::io::Result<()> {
        match self.state.status {
            Status::Disconnected | Status::Failed => self.connect(),
            Status::Connected => self.communicate(),
        }
    }

    fn communicate(&mut self) -> std::io::Result<()> {
        if self.state.reject {
            return self.try_clear_reject_condition();
        }

        if let Some(bytes) = self.channels.receive()? {
            self.transaction(bytes.ash_chunks()?)?;
        } else {
            self.handle_callbacks()?;
        }

        Ok(())
    }
}
