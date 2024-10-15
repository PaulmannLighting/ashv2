use crate::{AshFramed, Payload, Transceiver};
use serialport::SerialPort;
use std::sync::mpsc::{sync_channel, Receiver, SyncSender};
use std::task::Waker;

/// Create a pair of an [`AshFramed`] and a [`Transceiver`].
pub fn make_pair<const BUF_SIZE: usize, T>(
    serial_port: T,
    channel_size: usize,
    callback: Option<(SyncSender<Payload>, Receiver<Waker>)>,
) -> (AshFramed<BUF_SIZE>, Transceiver<T>)
where
    T: SerialPort,
{
    let (request_tx, request_rx) = sync_channel(channel_size);
    let (waker_tx, waker_rx) = sync_channel(channel_size);
    let transceiver = Transceiver::new(serial_port, request_rx, waker_rx, callback);
    let host = AshFramed::new(request_tx, waker_tx, channel_size);
    (host, transceiver)
}
