mod transaction;
mod worker;

use crate::protocol::host::transaction::Request;
use crate::{open, BaudRate, Error};
use log::{debug, error};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{channel, Sender};
use std::sync::Arc;
use std::thread;
use std::thread::JoinHandle;
use transaction::Transaction;
use worker::Worker;

#[derive(Debug)]
pub struct Host {
    path: String,
    baud_rate: BaudRate,
    sender: Option<Sender<Transaction>>,
    join_handle: Option<JoinHandle<()>>,
    terminate: Arc<AtomicBool>,
}

impl Host {
    #[must_use]
    pub fn new(path: String, baud_rate: BaudRate) -> Self {
        let mut host = Self {
            path,
            baud_rate,
            sender: None,
            join_handle: None,
            terminate: Arc::new(AtomicBool::new(false)),
        };
        host.spawn_worker();
        host
    }

    /// Communicate with the NCP.
    ///
    /// # Errors
    /// This function will return an [`Error`] if any error happen during communication.
    pub async fn communicate(&mut self, payload: &[u8]) -> Result<Arc<[u8]>, Error> {
        while self.terminate.load(Ordering::SeqCst) {
            error!("Worker has terminated. Attempting to restart.");
            self.restart_worker();
        }

        let transaction = Transaction::from(payload);

        if let Some(sender) = &self.sender {
            sender.send(transaction.clone())?;
        } else {
            transaction.resolve(Err(Error::WorkerNotRunning));
        }

        transaction.await
    }

    fn restart_worker(&mut self) {
        self.join_thread();
        self.terminate.store(false, Ordering::SeqCst);
        self.spawn_worker();
    }

    fn spawn_worker(&mut self) {
        let (sender, receiver) = channel::<Transaction>();
        let worker = Worker::new(
            open(&self.path, self.baud_rate.clone()).expect("Could not open serial port"),
            receiver,
            self.terminate.clone(),
        );
        self.join_handle = Some(thread::spawn(move || worker.spawn()));
        self.sender = Some(sender);
    }

    fn stop_worker(&mut self) {
        self.terminate.store(true, Ordering::SeqCst);

        if let Some(sender) = &self.sender {
            match sender.send(Transaction::new(Request::Terminate)) {
                Ok(_) => debug!("Successfully sent termination request."),
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
