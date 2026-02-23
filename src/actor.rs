use serialport::SerialPort;
use tokio::sync::mpsc::{Sender, channel};

pub use self::proxy::Proxy;
pub use self::receiver::Receiver;
pub use self::transmitter::Transmitter;
use crate::TryCloneNative;
use crate::actor::message::Message;
use crate::actor::tasks::Tasks;
use crate::types::Payload;

mod message;
mod proxy;
mod receiver;
mod tasks;
mod transmitter;

/// Actor that manages serial port communication.
#[derive(Debug)]
pub struct Actor<T> {
    receiver: Receiver<T>,
    transmitter: Transmitter<T>,
    sender: Sender<Message>,
}

impl<T> Actor<T>
where
    T: SerialPort,
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
    ) -> Result<Self, serialport::Error>
    where
        T: TryCloneNative,
    {
        let (tx_tx, tx_rx) = channel(message_queue_len);
        let receiver = Receiver::new(serial_port.try_clone_native()?, response, tx_tx.clone());
        let transmitter = Transmitter::new(serial_port, tx_rx, tx_tx.clone());
        Ok(Self {
            receiver,
            transmitter,
            sender: tx_tx,
        })
    }

    /// Spawns the actor's transmitter and receiver as asynchronous tasks.
    ///
    /// # Returns
    ///
    /// Returns a tuple of the tasks handler and the proxy.
    pub fn spawn(self) -> (Tasks, Proxy)
    where
        T: Sync + 'static,
    {
        (
            Tasks::spawn(self.transmitter, self.receiver, self.sender.clone()),
            self.sender.into(),
        )
    }
}
