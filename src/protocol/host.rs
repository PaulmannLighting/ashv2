mod transaction;
mod worker;

use crate::serial_port::open;
use crate::{BaudRate, Error};
use log::{debug, error};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{channel, Sender};
use std::sync::Arc;
use std::thread;
use std::thread::JoinHandle;
pub use transaction::{Request, ResultType, Transaction};
pub use worker::Worker;

#[derive(Debug)]
pub struct Host {
    path: String,
    baud_rate: BaudRate,
    sender: Option<Sender<Transaction>>,
    join_handle: Option<JoinHandle<()>>,
    terminate: Arc<AtomicBool>,
}

impl Host {
    /// Creates a new `ASHv2` host.
    ///
    /// # Errors
    /// Returns a [`serialport::Error`] if the serial port could not be created.
    pub fn new(path: String, baud_rate: BaudRate) -> Result<Self, serialport::Error> {
        let mut host = Self {
            path,
            baud_rate,
            sender: None,
            join_handle: None,
            terminate: Arc::new(AtomicBool::new(false)),
        };
        host.spawn_worker()?;
        Ok(host)
    }

    /// Communicate with the NCP.
    ///
    /// # Errors
    /// This function will return an [`Error`] if any error happen during communication.
    pub async fn communicate(&mut self, payload: &[u8]) -> ResultType {
        self.ensure_worker_is_running()?;
        let transaction = Transaction::from(payload);
        self.send_transaction(transaction.clone())?;
        transaction.await
    }

    /// Reset the NCP.
    ///
    /// # Errors
    /// This function will return an [`Error`] if any error happen during communication.
    pub async fn reset(&mut self) -> Result<(), Error> {
        self.ensure_worker_is_running()?;
        let transaction = Transaction::new(Request::Reset);
        self.send_transaction(transaction.clone())?;
        transaction.await.map(|_| ())
    }

    fn ensure_worker_is_running(&mut self) -> Result<(), Error> {
        while self.terminate.load(Ordering::SeqCst) {
            error!("Worker has terminated. Attempting to restart.");
            self.restart_worker()?;
        }

        Ok(())
    }

    fn send_transaction(&mut self, transaction: Transaction) -> Result<(), Error> {
        if let Some(sender) = &self.sender {
            sender.send(transaction)?;
        } else {
            transaction.resolve(Err(Error::WorkerNotRunning));
        }

        Ok(())
    }

    fn restart_worker(&mut self) -> Result<(), serialport::Error> {
        self.join_thread();
        self.terminate.store(false, Ordering::SeqCst);
        self.spawn_worker()
    }

    fn spawn_worker(&mut self) -> Result<(), serialport::Error> {
        let (sender, receiver) = channel::<Transaction>();
        let worker = Worker::new(
            open(&self.path, self.baud_rate.clone())?,
            receiver,
            self.terminate.clone(),
        );
        self.join_handle = Some(thread::spawn(move || worker.spawn()));
        self.sender = Some(sender);
        Ok(())
    }

    fn stop_worker(&mut self) {
        self.terminate.store(true, Ordering::SeqCst);

        if let Some(sender) = &self.sender {
            match sender.send(Transaction::new(Request::Terminate)) {
                Ok(_) => {
                    self.sender = None;
                    debug!("Successfully sent termination request.");
                }
                Err(error) => debug!("Failed to send termination request to worker: {error}"),
            }
        }

        self.join_thread();
    }

    fn join_thread(&mut self) {
        if let Some(join_handle) = self.join_handle.take() {
            if let Err(error) = join_handle.join() {
                error!("Thread did not terminate gracefully: {error:?}");
            }
        }
    }
}

impl Drop for Host {
    fn drop(&mut self) {
        self.stop_worker();
    }
}
