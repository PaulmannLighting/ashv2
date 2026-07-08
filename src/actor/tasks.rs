use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering::Relaxed;

use serialport::SerialPort;
use tokio::sync::mpsc::WeakSender;
use tokio::task::JoinHandle;

pub use self::error::Error;
use crate::actor::message::Message;

mod error;

/// Sender and receiver tasks wrapper to allow termination.
#[derive(Debug)]
pub struct Tasks<T> {
    handle: JoinHandle<T>,
    transmitter: JoinHandle<()>,
    receiver: JoinHandle<()>,
    sender: WeakSender<Message>,
    running: Arc<AtomicBool>,
}

impl<T> Tasks<T>
where
    T: SerialPort + Send + 'static,
{
    /// Create new tasks from a split serial port handle and actor components.
    pub(crate) const fn new(
        handle: JoinHandle<T>,
        transmitter: JoinHandle<()>,
        receiver: JoinHandle<()>,
        sender: WeakSender<Message>,
        running: Arc<AtomicBool>,
    ) -> Self {
        Self {
            handle,
            transmitter,
            receiver,
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
    pub async fn terminate(self) -> Result<T, Error> {
        self.running.store(false, Relaxed);
        self.receiver.await?;

        if let Some(sender) = self.sender.upgrade() {
            sender.send(Message::Terminate).await?;
        }

        self.transmitter.await?;
        let serial_port = self.handle.await?;
        Ok(serial_port)
    }
}
