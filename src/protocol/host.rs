mod connection_handler;
mod sent_frame;
mod worker;

use super::{CANCEL, FLAG, SUBSTITUTE, TIMEOUT, X_OFF, X_ON};
use crate::protocol::host::worker::{TaskQueue, Transaction, Worker};
use crate::Error;
use log::error;
use sent_frame::SentFrame;
use serialport::SerialPort;
use std::collections::HashMap;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering::SeqCst;
use std::sync::{Arc, Mutex};
use std::thread;
use std::thread::JoinHandle;

#[derive(Debug)]
pub struct Host {
    queue: Arc<Mutex<TaskQueue<Transaction>>>,
    ready: Arc<Mutex<HashMap<usize, Transaction>>>,
    join_handle: Option<JoinHandle<()>>,
    terminate: Arc<AtomicBool>,
}

impl Host {
    pub fn new<S>(serial_port: S) -> Self
    where
        for<'s> S: SerialPort + 's,
    {
        let queue = Arc::new(Mutex::new(TaskQueue::new()));
        let ready = Arc::new(Mutex::new(HashMap::new()));
        let terminate = Arc::new(AtomicBool::new(false));
        let worker = Worker::new(serial_port, terminate.clone(), queue.clone(), ready.clone());
        let join_handle = thread::spawn(move || worker.spawn());
        Self {
            queue,
            ready,
            join_handle: Some(join_handle),
            terminate,
        }
    }

    pub fn reset(&self) -> std::io::Result<Vec<u8>> {
        todo!()
    }

    pub async fn communicate(&self, payload: &[u8]) -> Result<Vec<u8>, Error> {
        todo!()
    }
}

impl Drop for Host {
    fn drop(&mut self) {
        self.terminate.store(true, SeqCst);

        if let Some(join_handle) = self.join_handle.take() {
            if let Err(error) = join_handle.join() {
                error!("Thread did not terminate gracefully: {error:?}");
            }
        }
    }
}
