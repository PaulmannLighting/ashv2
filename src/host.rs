mod listener;
mod transmitter;

use crate::protocol::{Command, Response};
use crate::util::NonPoisonedRwLock;
use crate::Error;
use listener::Listener;
use log::error;
use serialport::SerialPort;
use std::sync::atomic::Ordering::SeqCst;
use std::sync::atomic::{AtomicBool, AtomicU8};
use std::sync::mpsc::{channel, Sender};
use std::sync::{Arc, Mutex};
use std::thread::{spawn, JoinHandle};
use std::time::Duration;
use transmitter::Transmitter;

const SOCKET_TIMEOUT: Duration = Duration::from_millis(1);

type OptionalBytesSender = Option<Sender<Arc<[u8]>>>;

#[derive(Debug)]
pub struct Host<'cmd, S>
where
    S: SerialPort,
{
    serial_port: Arc<Mutex<S>>,
    running: Arc<AtomicBool>,
    command: Option<Mutex<Sender<Command<'cmd>>>>,
    listener_thread: Option<JoinHandle<OptionalBytesSender>>,
    transmitter_thread: Option<JoinHandle<()>>,
    callback: Option<Sender<Arc<[u8]>>>,
}

impl<'cmd, S> Host<'cmd, S>
where
    S: SerialPort,
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

    /// Creates and starts the host.
    ///
    /// # Errors
    /// Returns an [`Error`] if the host could not be started.
    ///
    /// # Panics
    /// This function may panic if any locks are poisoned.
    pub fn spawn(serial_port: S, callback: Option<Sender<Arc<[u8]>>>) -> Result<Self, Error>
    where
        Self: 'static,
    {
        let mut instance = Self::new(serial_port);
        instance.start(callback).map(|()| instance)
    }

    /// Communicate with the NCP, returning [`T::Result`].
    ///
    /// # Errors
    /// Returns [`T::Error`] if the transactions fails.
    ///
    /// # Panics
    /// This function will panic if the sender's mutex is poisoned.
    pub async fn communicate<'t: 'cmd, T>(&self, payload: &[u8]) -> Result<T::Result, T::Error>
    where
        Self: 'static,
        T: Clone + Default + Response + Sync + Send + 't,
    {
        let response = T::default();
        self.send(Command::new(Arc::from(payload), Arc::new(response.clone())))?;
        response.await
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
    pub fn start(&mut self, callback: Option<Sender<Arc<[u8]>>>) -> Result<(), Error>
    where
        Self: 'static,
    {
        self.serial_port
            .lock()
            .expect("Socket should not be poisoned.")
            .set_timeout(SOCKET_TIMEOUT)?;
        let (command_sender, command_receiver) = channel();
        let connected = Arc::new(AtomicBool::new(false));
        let handler = Arc::new(NonPoisonedRwLock::new(None));
        let ack_number = Arc::new(AtomicU8::new(0));
        let (listener, ack_receiver, nak_receiver) = Listener::create(
            self.serial_port.clone(),
            self.running.clone(),
            connected.clone(),
            handler.clone(),
            ack_number.clone(),
            callback,
        );
        let transmitter = Transmitter::new(
            self.serial_port.clone(),
            self.running.clone(),
            connected,
            command_receiver,
            handler,
            ack_number,
            ack_receiver,
            nak_receiver,
        );
        self.command = Some(Mutex::new(command_sender));
        self.running.store(true, SeqCst);
        self.listener_thread = Some(spawn(move || listener.run()));
        self.transmitter_thread = Some(spawn(move || transmitter.run()));
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

    fn send(&self, command: Command<'cmd>) -> Result<(), Error>
    where
        Self: 'static,
    {
        self.command.as_ref().map_or_else(
            || Err(Error::WorkerNotRunning),
            |channel| {
                channel
                    .lock()
                    .expect("Channel mutex should never be poisoned.")
                    .send(command)
                    .map_err(|_| Error::Terminated)
            },
        )
    }
}

impl<S> Drop for Host<'_, S>
where
    S: SerialPort,
{
    fn drop(&mut self) {
        self.stop();
    }
}
