use serialport::SerialPort;
use tokio::spawn;
use tokio::sync::mpsc::{self, channel};
use tokio::task::JoinHandle;

pub use self::proxy::Proxy;
pub use self::receiver::Receiver;
pub use self::transmitter::Transmitter;
use crate::TryCloneNative;
use crate::types::Payload;

mod message;
mod proxy;
mod receiver;
mod transmitter;

/// Actor that manages serial port communication.
#[derive(Debug)]
pub struct Actor<T> {
    receiver: Receiver<T>,
    transmitter: Transmitter<T>,
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
        rx_queue_len: usize,
        tx_queue_len: usize,
    ) -> Result<(Self, Proxy, mpsc::Receiver<Payload>), serialport::Error>
    where
        T: TryCloneNative,
    {
        let (rx_tx, rx_rx) = channel(rx_queue_len);
        let (tx_tx, tx_rx) = channel(tx_queue_len);
        let receiver = Receiver::new(serial_port.try_clone_native()?, rx_tx, tx_tx.clone());
        let transmitter = Transmitter::new(serial_port, tx_rx, tx_tx.clone());
        Ok((
            Self {
                receiver,
                transmitter,
            },
            Proxy::new(tx_tx),
            rx_rx,
        ))
    }

    /// Spawns the actor's transmitter and receiver as asynchronous tasks.
    pub fn spawn(self) -> (JoinHandle<()>, JoinHandle<()>)
    where
        T: Sync + 'static,
    {
        (spawn(self.transmitter.run()), spawn(self.receiver.run()))
    }
}
