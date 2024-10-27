use serialport::SerialPort;
use tokio::sync::mpsc::channel;

use crate::{Stream, Transceiver};

/// Create a pair of an [`AshFramed`] and a [`Transceiver`].
pub fn make_pair<T>(serial_port: T, channel_size: usize) -> (Stream, Transceiver<T>)
where
    T: SerialPort,
{
    let (request_tx, request_rx) = channel(channel_size);
    let (response_tx, response_rx) = channel(channel_size);
    let transceiver = Transceiver::new(serial_port, request_rx, response_tx);
    let host = Stream::new(request_tx, response_rx);
    (host, transceiver)
}
