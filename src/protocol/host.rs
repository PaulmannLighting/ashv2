mod connection_handler;
mod sent_frame;
mod transaction;
mod worker;

use super::{CANCEL, FLAG, SUBSTITUTE, TIMEOUT, X_OFF, X_ON};
use crate::Error;
use log::error;
use sent_frame::SentFrame;
use serialport::SerialPort;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{channel, Sender};
use std::sync::Arc;
use std::thread;
use std::thread::JoinHandle;
use transaction::Transaction;
use worker::Worker;

#[derive(Debug)]
pub struct Host {
    sender: Sender<Transaction>,
    join_handle: Option<JoinHandle<()>>,
    terminate: Arc<AtomicBool>,
}

impl Host {
    pub fn new<S>(serial_port: S) -> Self
    where
        for<'s> S: SerialPort + 's,
    {
        let terminate = Arc::new(AtomicBool::new(false));
        let (sender, receiver) = channel::<Transaction>();
        let worker = Worker::new(serial_port, receiver, terminate.clone());
        Self {
            sender,
            join_handle: Some(thread::spawn(move || worker.spawn())),
            terminate,
        }
    }

    /// Communicate with the NCP.
    ///
    /// # Errors
    /// This function will return an [`Error`] if any error happen during communication.
    pub async fn communicate(&mut self, payload: &[u8]) -> Result<Arc<[u8]>, Error> {
        let transaction = Transaction::new(payload.into());
        self.sender.send(transaction.clone())?;
        transaction.await
    }
}

impl Drop for Host {
    fn drop(&mut self) {
        self.terminate.store(true, Ordering::SeqCst);

        if let Some(join_handle) = self.join_handle.take() {
            if let Err(error) = join_handle.join() {
                error!("Thread did not terminate gracefully: {error:?}");
            }
        }
    }
}
