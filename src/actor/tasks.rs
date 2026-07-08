use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering::Relaxed;

use either::{Either, Left, Right};
use serialport::SerialPort;
use tokio::spawn;
use tokio::sync::mpsc::Sender;
use tokio::sync::mpsc::error::SendError;
use tokio::task::{JoinError, JoinHandle};

use crate::actor::message::Message;
use crate::actor::{Receiver, Transmitter};

/// Sender and receiver tasks wrapper to allow termination.
#[derive(Debug)]
pub struct Tasks<T> {
    handle: JoinHandle<T>,
    transmitter: JoinHandle<()>,
    receiver: JoinHandle<()>,
    sender: Sender<Message>,
    running: Arc<AtomicBool>,
}

impl<T> Tasks<T>
where
    T: SerialPort + Send + 'static,
{
    /// Create new tasks from a split serial port handle and actor components.
    pub(crate) fn spawn(
        handle: JoinHandle<T>,
        transmitter: Transmitter,
        receiver: Receiver,
        sender: Sender<Message>,
    ) -> Self {
        let running = Arc::new(AtomicBool::new(true));
        Self {
            handle,
            transmitter: spawn(transmitter.run()),
            receiver: spawn(receiver.run(running.clone())),
            sender,
            running,
        }
    }
}

impl<T> Tasks<T> {
    /// Terminate the tasks.
    ///
    /// # Errors
    ///
    /// Returns either
    /// - a [`SendError`] if sending the termination message fails, or
    /// - a [`JoinError`] if joining either task fails.
    pub async fn terminate(self) -> Result<T, Either<SendError<Message>, JoinError>> {
        self.running.store(false, Relaxed);
        self.receiver.await.map_err(Right)?;
        self.sender.send(Message::Terminate).await.map_err(Left)?;
        self.transmitter.await.map_err(Right)?;
        let serial_port = self.handle.await.map_err(Right)?;
        Ok(serial_port)
    }
}
