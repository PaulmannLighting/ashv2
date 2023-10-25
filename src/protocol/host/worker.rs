mod active_transaction;
mod task_queue;
mod transaction;

use crate::packet::ack::Ack;
use crate::packet::data::Data;
use crate::packet::rst::Rst;
use crate::packet::Packet;
use crate::protocol::host::worker::task_queue::TaskQueue;
use crate::protocol::FLAG;
use crate::Error;
use itertools::{Chunks, Itertools};
use log::error;
use serialport::SerialPort;
use std::cell::{Ref, RefCell};
use std::collections::{HashMap, VecDeque};
use std::iter::Copied;
use std::slice::Iter;
use std::sync::{Arc, Mutex};
pub use transaction::Transaction;

pub const ACK_TIMEOUTS: usize = 4;

#[derive(Debug)]
pub struct Worker<S>
where
    S: SerialPort,
{
    serial_port: RefCell<S>,
    queue: Arc<Mutex<TaskQueue<Transaction>>>,
    ready: Arc<Mutex<HashMap<usize, Transaction>>>,
    frame_number: RefCell<u8>,
    unacknowledged_data: RefCell<VecDeque<Data>>,
}

impl<S> Worker<S>
where
    S: SerialPort,
{
    #[must_use]
    pub fn new(
        serial_port: S,
        queue: Arc<Mutex<TaskQueue<Transaction>>>,
        ready: Arc<Mutex<HashMap<usize, Transaction>>>,
    ) -> Self {
        Self {
            serial_port: RefCell::new(serial_port),
            queue,
            ready,
            frame_number: RefCell::new(0),
            unacknowledged_data: RefCell::new(VecDeque::new()),
        }
    }

    pub fn spawn(self) {
        loop {
            if let Ok(mut queue) = self.queue.lock() {
                if let Some((id, transaction)) = queue.pop() {
                    self.process_transaction(id, transaction)
                }
            }
        }
    }

    fn process_transaction(&self, id: usize, mut transaction: Transaction) {
        let result = transaction
            .chunks()
            .and_then(|chunks| self.process_chunks(chunks.into_iter()));

        if let Err(error) = &result {
            self.recover_error(error);
        }

        transaction.resolve(result);
        self.ready
            .lock()
            .expect("could not lock ready map")
            .insert(id, transaction);
    }

    fn process_chunks(&self, mut chunks: Chunks<Copied<Iter<u8>>>) -> Result<Arc<[u8]>, Error> {
        loop {
            if self.unacknowledged_data.borrow().len() < ACK_TIMEOUTS - 1 {
                if let Some(chunk) = chunks.next() {
                    let data =
                        Data::try_from((self.next_frame_number(), chunk.collect_vec().into()))?;
                    self.send_frame(&data)?;
                    self.unacknowledged_data.borrow_mut().push_back(data);
                }
            }

            self.receive_frame()?;
        }
    }

    fn next_frame_number(&self) -> u8 {
        let frame_number = self.frame_number.take();
        self.frame_number.replace((frame_number + 1) % 8);
        frame_number
    }

    fn send_frame<F>(&self, frame: F) -> std::io::Result<()>
    where
        F: IntoIterator<Item = u8>,
    {
        let mut serial_port = self.serial_port.borrow_mut();

        for byte in frame.into_iter() {
            serial_port.write_all(&[byte])?;
        }

        serial_port.write_all(&[FLAG])
    }

    fn receive_frame(&self) -> Result<Packet, Error> {
        loop {}
    }

    fn recover_error(&self, error: &Error) {
        match error {
            Error::Io(error) => {
                error!("Attempting to recover from I/O error: {error}");
                let _ = self.send_frame(&Rst::default());
            }
            _ => todo!(),
        }
    }
}
