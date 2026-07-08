use std::sync::Arc;
use std::sync::atomic::AtomicBool;

use async_serialport::{AsyncSerialPort, WorkerFuture};
use serialport::SerialPort;
use tokio::sync::mpsc::{Sender, channel};

pub use self::handle::Handle;
pub use self::receiver::Receiver;
pub use self::transmitter::Transmitter;
pub use crate::actor::tasks::{ActorFuture, Error};
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
pub fn start<T>(
    serial_port: T,
    response: Sender<Payload>,
) -> (WorkerFuture<T>, ActorFuture<()>, ActorFuture<()>, Handle)
where
    T: SerialPort + 'static,
{
    let (sender, inbox) = channel(response.capacity());
    let (reader, writer, handle) = serial_port.split(response.capacity());
    let running = Arc::new(AtomicBool::new(true));
    let receiver = Box::pin(Receiver::new(reader, response, sender.clone()).run(running.clone()));
    let transmitter = Box::pin(Transmitter::new(writer, inbox, sender.downgrade()).run(running));

    (handle, transmitter, receiver, sender.into())
}
