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
pub struct Tasks {
    transmitter: JoinHandle<()>,
    receiver: JoinHandle<()>,
    sender: Sender<Message>,
    running: Arc<AtomicBool>,
}

impl Tasks {
    /// Crate new tasks.
    pub(crate) fn spawn<T>(
        transmitter: Transmitter<T>,
        receiver: Receiver<T>,
        sender: Sender<Message>,
    ) -> Self
    where
        T: SerialPort + Sync + 'static,
    {
        let running = Arc::new(AtomicBool::new(true));
        Self {
            transmitter: spawn(transmitter.run()),
            receiver: spawn(receiver.run(running.clone())),
            sender,
            running,
        }
    }

    /// Terminate the tasks.
    ///
    /// # Errors
    ///
    /// Returns either
    /// - a [`SendError`] if sending the termination message fails, or
    /// - a [`JoinError`] if joining either task fails.
    pub async fn terminate(self) -> Result<(), Either<SendError<Message>, JoinError>> {
        self.running.store(false, Relaxed);
        self.sender.send(Message::Terminate).await.map_err(Left)?;
        self.transmitter.await.map_err(Right)?;
        self.receiver.await.map_err(Right)?;
        Ok(())
    }
}
