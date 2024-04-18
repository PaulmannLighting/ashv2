use std::sync::atomic::Ordering::SeqCst;
use std::sync::atomic::{AtomicBool, AtomicU8};
use std::sync::mpsc::{channel, Sender};
use std::sync::{Arc, Mutex};
use std::thread::{spawn, JoinHandle};
use std::time::Duration;

use log::error;
use serialport::SerialPort;

use listener::Listener;
use transmitter::Transmitter;

use crate::packet::FrameBuffer;
use crate::protocol::{Command, Response};
use crate::util::NonPoisonedRwLock;
use crate::Error;

mod listener;
mod transmitter;

const SOCKET_TIMEOUT: Duration = Duration::from_millis(1);

#[derive(Debug)]
pub struct Host {
    running: Arc<AtomicBool>,
    command: Sender<Command>,
    listener_thread: Option<JoinHandle<()>>,
    transmitter_thread: Option<JoinHandle<()>>,
}

impl Host {
    /// Creates and starts the host.
    ///
    /// # Errors
    /// Returns an [`Error`] if the host could not be started.
    ///
    /// # Panics
    /// This function may panic if any locks are poisoned.
    pub fn spawn<S>(
        mut serial_port: S,
        callback: Option<Sender<FrameBuffer>>,
    ) -> Result<Self, Error>
    where
        for<'a> S: SerialPort + 'a,
    {
        let running = Arc::new(AtomicBool::new(true));
        serial_port.set_timeout(SOCKET_TIMEOUT)?;
        let serial_port = Arc::new(Mutex::new(serial_port));
        let (command_sender, command_receiver) = channel();
        let connected = Arc::new(AtomicBool::new(false));
        let handler = Arc::new(NonPoisonedRwLock::new(None));
        let ack_number = Arc::new(AtomicU8::new(0));
        let (listener, ack_receiver, nak_receiver) = Listener::new(
            serial_port.clone(),
            running.clone(),
            connected.clone(),
            handler.clone(),
            ack_number.clone(),
            callback,
        );
        let transmitter = Transmitter::new(
            serial_port,
            running.clone(),
            connected,
            command_receiver,
            handler,
            ack_number,
            ack_receiver,
            nak_receiver,
        );

        Ok(Self {
            command: command_sender,
            running,
            listener_thread: Some(spawn(move || listener.run())),
            transmitter_thread: Some(spawn(move || transmitter.run())),
        })
    }

    /// Communicate with the NCP, returning [`T::Result`](Response::Result).
    ///
    /// # Errors
    /// Returns [`T::Error`](Response::Error) if the transactions fails.
    ///
    /// # Panics
    /// This function will panic if the sender's mutex is poisoned.
    pub async fn communicate<T>(&self, payload: &[u8]) -> Result<T::Result, T::Error>
    where
        for<'a> T: Clone + Default + Response + Sync + Send + 'a,
    {
        let response = T::default();
        let clone = Arc::new(response.clone());
        self.command
            .send(Command::new(Arc::from(payload), clone))
            .map_err(|_| Error::Terminated)?;
        response.await
    }
}

impl Drop for Host {
    fn drop(&mut self) {
        self.running.store(false, SeqCst);

        if let Some(thread) = self.listener_thread.take() {
            thread.join().unwrap_or_else(|_| {
                error!("Failed to join listener thread.");
            });
        }

        if let Some(thread) = self.transmitter_thread.take() {
            thread.join().unwrap_or_else(|_| {
                error!("Failed to join transmitter thread.");
            });
        }
    }
}
