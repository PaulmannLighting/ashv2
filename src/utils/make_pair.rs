use crate::{AshFramed, Transceiver};
use serialport::SerialPort;
use std::sync::mpsc::{sync_channel, SyncSender};

/// Create a pair of an `AshFramed` and a `Transceiver`.
pub fn make_pair<const BUF_SIZE: usize, T>(
    serial_port: T,
    callback: Option<SyncSender<Box<[u8]>>>,
) -> (AshFramed<BUF_SIZE>, Transceiver<T>)
where
    T: SerialPort,
{
    let (request_tx, request_rx) = sync_channel(BUF_SIZE);
    let (waker_tx, waker_rx) = sync_channel(BUF_SIZE);
    let transceiver = Transceiver::new(serial_port, request_rx, waker_rx, callback);
    let host = AshFramed::new(request_tx, waker_tx);
    (host, transceiver)
}
