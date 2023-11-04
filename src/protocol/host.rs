mod command;
mod listener;
mod transmitter;

use crate::Error;
use command::Command;
use command::ResetResponse;
pub use command::{Event, HandleResult, Response};
use listener::Listener;
use log::error;
use serialport::SerialPort;
use std::future::Future;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering::SeqCst;
use std::sync::mpsc::{channel, SendError, Sender};
use std::sync::{Arc, Mutex};
use std::thread::{spawn, JoinHandle};
use transmitter::Transmitter;

type OptionalBytesSender = Option<Sender<Arc<[u8]>>>;

#[derive(Debug)]
pub struct Host<S>
where
    S: SerialPort,
{
    serial_port: S,
    running: Arc<AtomicBool>,
    command: Option<Sender<Command>>,
    listener_thread: Option<JoinHandle<OptionalBytesSender>>,
    transmitter_thread: Option<JoinHandle<()>>,
    callback: Option<Sender<Arc<[u8]>>>,
}

impl<S> Host<S>
where
    S: SerialPort,
{
    /// Creates a new `ASHv2` host.
    pub fn new(serial_port: S) -> Self {
        Self {
            serial_port,
            running: Arc::new(AtomicBool::new(false)),
            command: None,
            listener_thread: None,
            transmitter_thread: None,
            callback: None,
        }
    }

    /// Communicate with the NCP.
    ///
    /// # Errors
    /// Returns an error if the transactions fails.
    pub async fn communicate<R, T, E>(&mut self, payload: &[u8]) -> <R as Future>::Output
    where
        R: Clone + Default + Future<Output = Result<T, E>> + Response<Arc<[u8]>> + 'static,
        E: From<Error> + From<SendError<Command>>,
    {
        if let Some(channel) = &mut self.command {
            let response = R::default();
            let command = Command::new_data(payload, response.clone());

            if let Err(error) = channel.send(command) {
                <R as Future>::Output::Err(error.into())
            } else {
                response.await
            }
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
    pub fn is_running(&self) -> bool {
        self.running.load(SeqCst)
            || self.listener_thread.is_some()
            || self.transmitter_thread.is_some()
    }

    /// Starts the host..
    ///
    /// # Errors
    /// Returns an [`Error`] if the host is already running or the serial port cannot be cloned..
    pub fn start(&mut self, callback: Option<Sender<Arc<[u8]>>>) -> Result<(), Error> {
        if self.is_running() {
            return Err(Error::AlreadyRunning);
        }

        let (command_sender, command_receiver) = channel();
        let connected = Arc::new(AtomicBool::new(false));
        let current_command = Arc::new(Mutex::new(None));
        let (listener, ack_receiver, nak_receiver) = Listener::create(
            self.serial_port.try_clone()?,
            self.running.clone(),
            connected.clone(),
            current_command.clone(),
            callback,
        );
        let transmitter = Transmitter::new(
            self.serial_port.try_clone()?,
            self.running.clone(),
            connected,
            command_receiver,
            current_command,
            ack_receiver,
            nak_receiver,
        );
        self.command = Some(command_sender);
        self.running.store(true, SeqCst);
        self.listener_thread = Some(spawn(|| listener.run()));
        self.transmitter_thread = Some(spawn(|| transmitter.spawn()));
        Ok(())
    }

    fn stop(&mut self) {
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

    /// Restarts the host.
    ///
    /// # Errors
    /// Returns an [`Error`] on I/O, protocol or parsing errors.
    pub fn restart(&mut self) -> Result<(), Error> {
        self.stop();
        let callback = self.callback.take();
        self.start(callback)
    }
}

impl<S> Drop for Host<S>
where
    S: SerialPort,
{
    fn drop(&mut self) {
        self.stop();
    }
}
