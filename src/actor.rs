use async_serialport::AsyncSerialPort;
use serialport::SerialPort;
use tokio::sync::mpsc::{Sender, channel};
use tokio::task::JoinHandle;

pub use self::handle::Handle;
pub use self::receiver::Receiver;
pub use self::transmitter::Transmitter;
use crate::actor::message::Message;
pub use crate::actor::tasks::Tasks;
use crate::types::Payload;

mod handle;
mod message;
mod receiver;
mod tasks;
mod transmitter;

/// Actor that manages serial port communication.
#[derive(Debug)]
pub struct Actor<T> {
    handle: JoinHandle<T>,
    receiver: Receiver,
    transmitter: Transmitter,
    sender: Sender<Message>,
}

impl<T> Actor<T>
where
    T: SerialPort + 'static,
{
    /// Creates a new actor with the given serial port and queue lengths.
    ///
    /// # Errors
    ///
    /// Returns a [`serialport::Error`] if the serial port cannot be cloned.
    pub fn new(
        serial_port: T,
        response: Sender<Payload>,
        message_queue_len: usize,
    ) -> Result<Self, serialport::Error> {
        let (tx_tx, tx_rx) = channel(message_queue_len);
        let (reader, writer, handle) = serial_port.split(message_queue_len);
        let receiver = Receiver::new(reader, response, tx_tx.clone());
        let transmitter = Transmitter::new(writer, tx_rx, tx_tx.downgrade());
        Ok(Self {
            handle,
            receiver,
            transmitter,
            sender: tx_tx,
        })
    }

    /// Spawns the actor's transmitter and receiver as asynchronous tasks.
    ///
    /// # Returns
    ///
    /// Returns a tuple of the tasks handler and actor handle.
    pub fn spawn(self) -> (Tasks<T>, Handle) {
        (
            Tasks::spawn(
                self.handle,
                self.transmitter,
                self.receiver,
                self.sender.clone(),
            ),
            self.sender.into(),
        )
    }
}
