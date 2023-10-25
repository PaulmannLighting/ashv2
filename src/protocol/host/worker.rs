use super::transaction::Transaction;
use crate::packet::ack::Ack;
use crate::packet::data::Data;
use crate::packet::nak::Nak;
use crate::packet::rst::Rst;
use crate::packet::Packet;
use crate::protocol::FLAG;
use crate::Error;
use itertools::{Chunk, Itertools};
use log::error;
use serialport::SerialPort;
use std::cell::RefCell;
use std::collections::VecDeque;
use std::iter::Copied;
use std::slice::Iter;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering::SeqCst;
use std::sync::{Arc, Mutex};

pub const ACK_TIMEOUTS: usize = 4;

#[derive(Debug)]
pub struct Worker<S>
where
    S: SerialPort,
{
    /// Shared state
    serial_port: RefCell<S>,
    queue: Arc<Mutex<VecDeque<Transaction>>>,
    terminate: Arc<AtomicBool>,
    /// Local state
    frame_number: RefCell<u8>,
    ack_number: RefCell<u8>,
    unacknowledged_data: RefCell<VecDeque<Data>>,
    buffer: RefCell<Vec<u8>>,
}

impl<S> Worker<S>
where
    S: SerialPort,
{
    #[must_use]
    pub fn new(
        serial_port: S,
        queue: Arc<Mutex<VecDeque<Transaction>>>,
        terminate: Arc<AtomicBool>,
    ) -> Self {
        Self {
            serial_port: RefCell::new(serial_port),
            queue,
            terminate,
            frame_number: RefCell::new(0),
            ack_number: RefCell::new(0),
            unacknowledged_data: RefCell::new(VecDeque::new()),
            buffer: RefCell::new(Vec::new()),
        }
    }

    pub fn spawn(self) {
        while !self.terminate.load(SeqCst) {
            if let Ok(mut queue) = self.queue.lock() {
                if let Some(transaction) = queue.pop_back() {
                    self.process_transaction(transaction);
                }
            }
        }
    }

    fn process_transaction(&self, mut transaction: Transaction) {
        let result = transaction
            .chunks()
            .and_then(|chunks| self.process_chunks(chunks.into_iter().collect_vec()));

        if let Err(error) = &result {
            self.recover_error(error);
        }

        transaction.resolve(result);
    }

    fn process_chunks(&self, mut chunks: Vec<Chunk<Copied<Iter<u8>>>>) -> Result<Arc<[u8]>, Error> {
        while !self.terminate.load(SeqCst) {
            self.push_chunks(&mut chunks)?;

            match self.receive_frame()? {
                Packet::Data(data) => {
                    todo!("validate packet and push data to buffer")
                    // if this was the last expected data packet, return Ok(buffer.into()).
                }
                _ => todo!("Handle other frame types"),
            }
        }

        Err(Error::Terminated)
    }

    fn next_frame_number(&self) -> u8 {
        let frame_number = self.frame_number.take();
        self.frame_number.replace((frame_number + 1) % 8);
        frame_number
    }

    fn push_chunks(&self, chunks: &mut Vec<Chunk<Copied<Iter<u8>>>>) -> Result<(), Error> {
        while self.unacknowledged_data.borrow().len() < ACK_TIMEOUTS - 1 {
            if let Some(chunk) = chunks.pop() {
                let data = Data::try_from((self.next_frame_number(), chunk.collect_vec().into()))?;
                self.send_frame(&data)?;
                self.unacknowledged_data.borrow_mut().push_back(data);
            } else {
                break;
            }
        }

        Ok(())
    }

    fn ack_sent_data(&self, ack_num: u8) {
        self.unacknowledged_data
            .borrow_mut()
            .retain(|data| data.frame_num() != ack_num);
    }

    fn send_ack(&self, ack_num: u8) -> std::io::Result<()> {
        self.send_frame(&Ack::from(ack_num))
    }

    fn send_nak(&self, ack_num: u8) -> std::io::Result<()> {
        self.send_frame(&Nak::from(ack_num))
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
        todo!("Implement receiving of frames")
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
