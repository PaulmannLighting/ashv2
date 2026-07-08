use std::sync::Arc;
use std::sync::atomic::AtomicBool;

use async_serialport::AsyncSerialPort;
use serialport::SerialPort;
use tokio::spawn;
use tokio::sync::mpsc::{Sender, channel};
use tokio::task::JoinHandle;

pub use self::handle::Handle;
pub use self::receiver::Receiver;
pub use self::transmitter::Transmitter;
use crate::actor::message::Message;
pub use crate::actor::tasks::{Error, Tasks};
use crate::types::Payload;

mod handle;
mod message;
mod receiver;
mod tasks;
mod transmitter;

/// Actor that manages serial port communication.
#[derive(Debug)]
pub struct Actor<T> {
    handle: JoinHandle<T>,
    receiver: Receiver,
    transmitter: Transmitter,
    sender: Sender<Message>,
}

impl<T> Actor<T>
where
    T: SerialPort + 'static,
{
    /// Creates a new actor with the given serial port and queue lengths.
    ///
    /// # Errors
    ///
    /// Returns a [`serialport::Error`] if the serial port cannot be cloned.
    pub fn spawn(serial_port: T, response: Sender<Payload>) -> (Tasks<T>, Handle) {
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
}
