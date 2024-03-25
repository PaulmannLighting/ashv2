mod command;
mod listener;
mod transmitter;

use crate::Error;
use command::{Command, ResetResponse};
pub use command::{Event, HandleResult, Handler, Response};
use listener::Listener;
use log::error;
use serialport::SerialPort;
use std::sync::atomic::Ordering::SeqCst;
use std::sync::atomic::{AtomicBool, AtomicU8};
use std::sync::mpsc::{channel, Sender};
use std::sync::{Arc, Mutex, RwLock};
use std::thread::{spawn, JoinHandle};
use std::time::Duration;
use transmitter::Transmitter;

const SOCKET_TIMEOUT: Duration = Duration::from_millis(1);

type OptionalBytesSender = Option<Sender<Arc<[u8]>>>;

#[derive(Debug)]
pub struct Host<S>
where
    for<'s> S: SerialPort + 's,
{
    serial_port: Arc<Mutex<S>>,
    running: Arc<AtomicBool>,
    command: Option<Sender<Command>>,
    listener_thread: Option<JoinHandle<OptionalBytesSender>>,
    transmitter_thread: Option<JoinHandle<()>>,
    callback: Option<Sender<Arc<[u8]>>>,
}

impl<S> Host<S>
where
    for<'s> S: SerialPort + 's,
{
    /// Creates a new `ASHv2` host.
    #[must_use]
    pub fn new(serial_port: S) -> Self {
        Self {
            serial_port: Arc::new(Mutex::new(serial_port)),
            running: Arc::new(AtomicBool::new(false)),
            command: None,
            listener_thread: None,
            transmitter_thread: None,
            callback: None,
        }
    }

    /// Communicate with the NCP, returning [`T::Result`].
    ///
    /// # Errors
    /// Returns [`T::Error`] if the transactions fails.
    pub async fn communicate<T>(&mut self, payload: &[u8]) -> Result<T::Result, T::Error>
    where
        for<'r> T: Clone + Default + Response + 'r,
    {
        if let Some(channel) = &mut self.command {
            let response = T::default();
            channel.send(Command::new(payload, response.clone()))?;
            response.await
        } else {
            Err(Error::WorkerNotRunning.into())
        }
    }

    /// Reset the NCP.
    ///
    /// # Errors
    /// Returns an [`Error`] on I/O, protocol or parsing errors.
    pub async fn reset(&mut self) -> Result<(), Error> {
        if let Some(channel) = &mut self.command {
            let response = ResetResponse::default();
            channel.send(Command::Reset(response.clone()))?;
            response.await
        } else {
            Err(Error::WorkerNotRunning)
        }
    }

    /// Queries whether the host is running.
    #[must_use]
    pub fn is_running(&self) -> bool {
        self.running.load(SeqCst)
            || self.listener_thread.is_some()
            || self.transmitter_thread.is_some()
    }

    /// Starts the host.
    ///
    /// # Errors
    /// Returns an [`Error`] if the host could not be started.
    ///
    /// # Panics
    /// This function may panic if any locks are poisoned.
    pub fn start(&mut self, callback: Option<Sender<Arc<[u8]>>>) -> Result<(), Error> {
        self.serial_port
            .lock()
            .expect("Socket should not be poisoned.")
            .set_timeout(SOCKET_TIMEOUT)?;
        let (command_sender, command_receiver) = channel();
        let connected = Arc::new(AtomicBool::new(false));
        let current_command = Arc::new(RwLock::new(None));
        let ack_number = Arc::new(AtomicU8::new(0));
        let (listener, ack_receiver, nak_receiver) = Listener::create(
            self.serial_port.clone(),
            self.running.clone(),
            connected.clone(),
            current_command.clone(),
            ack_number.clone(),
            callback,
        );
        let transmitter = Transmitter::new(
            self.serial_port.clone(),
            self.running.clone(),
            connected,
            command_receiver,
            current_command,
            ack_number,
            ack_receiver,
            nak_receiver,
        );
        self.command = Some(command_sender);
        self.running.store(true, SeqCst);
        self.listener_thread = Some(spawn(|| listener.run()));
        self.transmitter_thread = Some(spawn(|| transmitter.run()));
        Ok(())
    }

    pub fn stop(&mut self) {
        self.running.store(false, SeqCst);

        if let Some(listener_thread) = self.listener_thread.take() {
            self.callback = listener_thread.join().unwrap_or_else(|_| {
                error!("Failed to join listener thread.");
                None
            });
        }

        if let Some(transmitter_thread) = self.transmitter_thread.take() {
            transmitter_thread
                .join()
                .unwrap_or_else(|_| error!("Failed to join transmitter thread."));
        }

        drop(self.command.take());
    }
}

impl<S> Drop for Host<S>
where
    for<'s> S: SerialPort + 's,
{
    fn drop(&mut self) {
        self.stop();
    }
}
