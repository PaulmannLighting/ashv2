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
    // Shared state
    queue: Arc<Mutex<VecDeque<Transaction>>>,
    terminate: Arc<AtomicBool>,
    // Local state
    serial_port: S,
    frame_number: u8,
    ack_number: u8,
    unacknowledged_data: VecDeque<Data>,
    buffer: Vec<u8>,
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
            queue,
            terminate,
            serial_port,
            frame_number: 0,
            ack_number: 0,
            unacknowledged_data: VecDeque::new(),
            buffer: Vec::new(),
        }
    }

    pub fn spawn(mut self) {
        while !self.terminate.load(SeqCst) {
            if let Some(transaction) = self.next_transaction() {
                self.process_transaction(transaction);
            }
        }
    }

    fn next_transaction(&mut self) -> Option<Transaction> {
        if let Ok(mut queue) = self.queue.lock() {
            if let Some(next_transaction) = queue.pop_back() {
                return Some(next_transaction);
            }
        }

        None
    }

    fn process_transaction(&mut self, mut transaction: Transaction) {
        let result = transaction
            .chunks()
            .and_then(|chunks| self.process_chunks(chunks.into_iter().collect_vec()));

        if let Err(error) = &result {
            self.recover_error(error);
        }

        transaction.resolve(result);
    }

    fn process_chunks(
        &mut self,
        mut chunks: Vec<Chunk<Copied<Iter<u8>>>>,
    ) -> Result<Arc<[u8]>, Error> {
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

    fn next_frame_number(&mut self) -> u8 {
        let frame_number = self.frame_number;
        self.frame_number = (frame_number + 1) % 8;
        frame_number
    }

    fn push_chunks(&mut self, chunks: &mut Vec<Chunk<Copied<Iter<u8>>>>) -> Result<(), Error> {
        while self.unacknowledged_data.len() < ACK_TIMEOUTS - 1 {
            if let Some(chunk) = chunks.pop() {
                let data = Data::try_from((self.next_frame_number(), chunk.collect_vec().into()))?;
                self.send_frame(&data)?;
                self.unacknowledged_data.push_back(data);
            } else {
                break;
            }
        }

        Ok(())
    }

    fn ack_sent_data(&mut self, ack_num: u8) {
        self.unacknowledged_data
            .retain(|data| data.frame_num() != ack_num);
    }

    fn send_ack(&mut self, ack_num: u8) -> std::io::Result<()> {
        self.send_frame(&Ack::from(ack_num))
    }

    fn send_nak(&mut self, ack_num: u8) -> std::io::Result<()> {
        self.send_frame(&Nak::from(ack_num))
    }

    fn send_frame<F>(&mut self, frame: F) -> std::io::Result<()>
    where
        F: IntoIterator<Item = u8>,
    {
        for byte in frame {
            self.serial_port.write_all(&[byte])?;
        }

        self.serial_port.write_all(&[FLAG])
    }

    fn receive_frame(&mut self) -> Result<Packet, Error> {
        todo!("Implement receiving of frames")
    }

    fn recover_error(&mut self, error: &Error) {
        match error {
            Error::Io(error) => {
                error!("Attempting to recover from I/O error: {error}");
                let _ = self.send_frame(&Rst::default());
            }
            _ => todo!(),
        }
    }
}
