use std::sync::Arc;
use std::sync::atomic::AtomicBool;

use async_serialport::AsyncSerialPort;
use serialport::SerialPort;
use tokio::spawn;
use tokio::sync::mpsc::{Sender, channel};

pub use self::handle::Handle;
pub use self::receiver::Receiver;
pub use self::transmitter::Transmitter;
pub use crate::actor::tasks::{Error, Tasks};
use crate::types::Payload;

mod handle;
mod message;
mod receiver;
mod tasks;
mod transmitter;

/// Start the `ASHv2` actor tasks for the given serial port.
///
/// The response channel receives inbound `DATA` payloads from the NCP. Its capacity is also
/// used for the actor's internal message queue.
pub fn start<T>(serial_port: T, response: Sender<Payload>) -> (Tasks<T>, Handle)
where
    T: SerialPort + 'static,
{
    let (sender, inbox) = channel(response.capacity());
    let (reader, writer, handle) = serial_port.split(response.capacity());
    let running = Arc::new(AtomicBool::new(true));
    let receiver = spawn(Receiver::new(reader, response, sender.clone()).run(running.clone()));
    let transmitter = spawn(Transmitter::new(writer, inbox, sender.downgrade()).run());

    (
        Tasks::new(handle, transmitter, receiver, sender.downgrade(), running),
        sender.into(),
    )
}
