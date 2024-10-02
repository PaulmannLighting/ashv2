mod buffers;
mod constants;
mod implementations;
mod retransmit;
mod state;

use crate::channels::Channels;
use crate::protocol::AshChunks;
use crate::request::Request;
use crate::status::Status;
use crate::transceiver::buffers::Buffers;
use crate::transceiver::state::State;
use serialport::SerialPort;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering::Relaxed;
use std::sync::mpsc::{Receiver, Sender};
use std::sync::Arc;

/// `ASHv2` transceiver.
///
/// The transceiver is responsible for handling the communication between the host and the NCP.
/// It is supposed to be run in a separate thread.
/// A [`Host`](crate::Host) can be used to communicate with the NCP via the transceiver.
///
/// # Usage
///
/// ```
/// use std::sync::Arc;
/// use std::sync::atomic::AtomicBool;
/// use std::sync::atomic::Ordering::Relaxed;
/// use std::sync::mpsc::channel;
/// use std::thread::spawn;
/// use serialport::FlowControl;
/// use tokio::task::futures;
/// use ashv2::{open, BaudRate, CommunicateSync, Host, Transceiver};
///
/// match open("/dev/ttyUSB0", BaudRate::RstCts, FlowControl::Software) {
///     Ok(serial_port) => {let (sender, receiver) = channel();
///         let transceiver = Transceiver::new(serial_port, receiver, None);
///         let running = Arc::new(AtomicBool::new(true));
///         let running_transceiver = running.clone();
///         let _thread_handle = spawn(move || transceiver.run(running_transceiver));
///         let host = Host::from(sender);
///
///         let version_command = &[0x00, 0x01, 0x02, 0x03];
///
///         match host.communicate(version_command) {
///             Ok(response) => println!("{response:?}"),
///             Err(error) => eprintln!("{error}"),
///         }
///
///         running.store(false, Relaxed);
///     },
///     Err(error) => eprintln!("{error}"),
/// }
///
/// ```
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
                self.handle_io_error(error);
            }
        }
    }

    fn main(&mut self) -> std::io::Result<()> {
        match self.state.status {
            Status::Disconnected | Status::Failed => Ok(self.connect()?),
            Status::Connected => self.communicate(),
        }
    }

    fn communicate(&mut self) -> std::io::Result<()> {
        if let Some(bytes) = self.channels.receive()? {
            self.transaction(bytes.ash_chunks()?)
        } else {
            self.handle_callbacks()
        }
    }
}
