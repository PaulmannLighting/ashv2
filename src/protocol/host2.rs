mod command;
mod listener;
mod transmitter;

use crate::protocol::host2::command::{ResetResponse, Response};
use crate::protocol::host2::listener::Listener;
use crate::protocol::host2::transmitter::Transmitter;
use crate::Error;
use command::Command;
use log::error;
use serialport::SerialPort;
use std::future::Future;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering::SeqCst;
use std::sync::mpsc::{channel, SendError, Sender};
use std::sync::{Arc, Mutex};
use std::thread::{spawn, JoinHandle};

#[derive(Debug)]
pub struct Host<S>
where
    S: SerialPort,
{
    serial_port: S,
    running: Arc<AtomicBool>,
    command: Option<Sender<Command>>,
    listener_thread: Option<JoinHandle<Option<Sender<Arc<[u8]>>>>>,
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
    pub async fn communicate<T>(&mut self, payload: &[u8]) -> <T as Future>::Output
    where
        T: Clone + Default + Future + Response<Arc<[u8]>> + 'static,
        <T as Future>::Output: From<Error> + From<SendError<Command>>,
    {
        if let Some(channel) = &mut self.command {
            let response = T::default();
            let command = Command::new_data(payload, response.clone());

            if let Err(error) = channel.send(command) {
                <T as Future>::Output::from(error)
            } else {
                response.await
            }
        } else {
            Error::WorkerNotRunning.into()
        }
    }

    /// Reset the NCP.
    ///
    /// # Errors
    /// This function will return an [`Error`] if any error happen during communication.
    pub async fn reset(&mut self) -> Result<(), Error> {
        if let Some(channel) = &mut self.command {
            let response = ResetResponse::default();
            channel.send(Command::Reset(response.clone()))?;
            response.await
        } else {
            Err(Error::WorkerNotRunning)
        }
    }

    pub fn is_running(&self) -> bool {
        self.running.load(SeqCst)
            || self.listener_thread.is_some()
            || self.transmitter_thread.is_none()
    }

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
            connected.clone(),
            command_receiver,
            current_command,
            ack_receiver,
            nak_receiver,
        );
        self.command = Some(command_sender);
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
