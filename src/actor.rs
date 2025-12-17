use serialport::SerialPort;
use tokio::sync::mpsc::{self, channel};

pub use self::receiver::Receiver;
pub use self::transmitter::Transmitter;
use crate::Payload;
use crate::actor::message::Message;

mod message;
mod receiver;
mod transmitter;

pub struct Actor {
    receiver: Receiver<Box<dyn SerialPort>>,
    transmitter: Transmitter<Box<dyn SerialPort>>,
}

impl Actor {
    pub fn new<T>(
        serial_port: T,
        rx_queue_len: usize,
        tx_queue_len: usize,
    ) -> Result<(Self, mpsc::Sender<Message>, mpsc::Receiver<Payload>), serialport::Error>
    where
        T: SerialPort + 'static,
    {
        let (rx_tx, rx_rx) = channel(rx_queue_len);
        let (tx_tx, tx_rx) = channel(tx_queue_len);
        let receiver = Receiver::new(serial_port.try_clone()?, rx_tx, tx_tx.clone());
        let transmitter = Transmitter::new(
            Box::<dyn SerialPort>::from(Box::new(serial_port)),
            tx_rx,
            tx_tx.clone(),
        );
        Ok((
            Self {
                receiver,
                transmitter,
            },
            tx_tx,
            rx_rx,
        ))
    }
}
