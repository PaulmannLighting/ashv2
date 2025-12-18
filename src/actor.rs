use serialport::{SerialPort, TTYPort};
use tokio::spawn;
use tokio::sync::mpsc::{self, channel};
use tokio::task::JoinHandle;

pub use self::proxy::{Error, Proxy};
pub use self::receiver::Receiver;
pub use self::transmitter::Transmitter;
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

impl Actor<TTYPort> {
    /// Creates a new actor with the given serial port and queue lengths.
    ///
    /// # Errors
    ///
    /// Returns a [`serialport::Error`] if the serial port cannot be cloned.
    pub fn new(
        serial_port: TTYPort,
        rx_queue_len: usize,
        tx_queue_len: usize,
    ) -> Result<(Self, Proxy, mpsc::Receiver<Payload>), serialport::Error> {
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
}

impl<T> Actor<T>
where
    T: SerialPort + Sync + 'static,
{
    /// Spawns the actor's transmitter and receiver as asynchronous tasks.
    pub fn spawn(self) -> (JoinHandle<()>, JoinHandle<()>) {
        let transmitter_handle = spawn(async move {
            self.transmitter.run().await;
        });
        let receiver_handle = spawn(async move {
            self.receiver.run().await;
        });
        (transmitter_handle, receiver_handle)
    }
}
