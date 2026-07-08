use std::future::Future;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;

use async_serialport::{AsyncSerialPort, Worker};
use serialport::SerialPort;
use tokio::sync::mpsc::{Sender, channel};

pub use self::handle::Handle;
pub use self::receiver::Receiver;
pub use self::transmitter::Transmitter;
pub use crate::actor::tasks::{Error, Futures};
use crate::types::Payload;

mod handle;
mod message;
mod receiver;
mod tasks;
mod transmitter;

/// Create the `ASHv2` actor futures for the given serial port.
///
/// The response channel receives inbound `DATA` payloads from the NCP. Its capacity is also
/// used for the actor's internal message queue.
///
/// Returns the user-facing [`Handle`] and named [`Futures`] that the caller must spawn or
/// otherwise poll on their async runtime.
pub fn start<T>(
    serial_port: T,
    response: Sender<Payload>,
) -> (
    Handle,
    Futures<
        Worker<T>,
        impl Future<Output = ()> + Send + 'static,
        impl Future<Output = ()> + Send + 'static,
    >,
)
where
    T: SerialPort + 'static,
{
    let (sender, inbox) = channel(response.capacity());
    let (reader, writer, serial_worker) = serial_port.split(response.capacity());
    let running = Arc::new(AtomicBool::new(true));
    let receiver = Receiver::new(reader, response, sender.clone()).run(running.clone());
    let transmitter = Transmitter::new(writer, inbox, sender.downgrade()).run(running);
    let futures = Futures::new(serial_worker, transmitter, receiver);

    (sender.into(), futures)
}
