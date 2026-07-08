use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering::Relaxed;

use async_serialport::WorkerFuture;
use tokio::sync::mpsc::WeakSender;

pub use self::error::Error;
use crate::actor::message::Message;

mod error;

/// Boxed actor future returned by [`crate::start`].
pub type ActorFuture<T> = Pin<Box<dyn Future<Output = T> + Send + 'static>>;

/// `ASHv2` futures that must be driven by the caller's async runtime.
///
/// Spawn or otherwise poll all three futures to run the serial worker, transmitter,
/// and receiver. Use [`Shutdown`] to request graceful termination.
pub struct Futures<T> {
    /// Future that drives the blocking serial-port worker and returns the serial port.
    pub serial_port: WorkerFuture<T>,
    /// Future that drives outbound `ASHv2` frame transmission.
    pub transmitter: ActorFuture<()>,
    /// Future that drives inbound `ASHv2` frame reception.
    pub receiver: ActorFuture<()>,
    /// Handle used to request graceful termination of the receiver and transmitter.
    pub shutdown: Shutdown,
}

impl<T> Futures<T> {
    /// Create actor futures from split serial port and actor components.
    pub(crate) const fn new(
        serial_port: WorkerFuture<T>,
        transmitter: ActorFuture<()>,
        receiver: ActorFuture<()>,
        shutdown: Shutdown,
    ) -> Self {
        Self {
            serial_port,
            transmitter,
            receiver,
            shutdown,
        }
    }
}

/// Handle used to request graceful termination of the actor futures.
#[derive(Clone, Debug)]
pub struct Shutdown {
    sender: WeakSender<Message>,
    running: Arc<AtomicBool>,
}

impl Shutdown {
    /// Create a new shutdown handle.
    pub(crate) const fn new(sender: WeakSender<Message>, running: Arc<AtomicBool>) -> Self {
        Self { sender, running }
    }

    /// Request actor termination.
    ///
    /// # Errors
    ///
    /// Returns [`Error`] if sending the termination message to the transmitter fails.
    pub async fn terminate(&self) -> Result<(), Error> {
        self.running.store(false, Relaxed);

        if let Some(sender) = self.sender.upgrade() {
            sender.send(Message::Terminate).await?;
        }

        Ok(())
    }
}
