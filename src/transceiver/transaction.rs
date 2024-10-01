use crate::ash_read::AshRead;
use crate::ash_write::AshWrite;
use crate::frame::Frame;
use crate::packet::{Ack, Data, Nak, Packet};
use crate::protocol::{AshChunks, Stuff, FLAG};
use crate::transceiver::buffers::Buffers;
use crate::transceiver::channels::Channels;
use crate::transceiver::state::State;
use crate::wrapping_u3::WrappingU3;
use log::{debug, trace, warn};
use serialport::TTYPort;
use std::io::{Error, ErrorKind, Write};
use std::slice::Chunks;

/// Transaction processor.
#[derive(Debug)]
pub struct Transaction<'a> {
    serial_port: &'a mut TTYPort,
    channels: &'a mut Channels,
    buffers: &'a mut Buffers,
    state: &'a mut State,
    chunks: Chunks<'a, u8>,
}

impl<'a> Transaction<'a> {
    /// Creates a new transaction processor.
    #[must_use]
    pub fn new(
        serial_port: &'a mut TTYPort,
        channels: &'a mut Channels,
        buffers: &'a mut Buffers,
        state: &'a mut State,
        chunks: Chunks<'a, u8>,
    ) -> Self {
        Self {
            serial_port,
            channels,
            buffers,
            state,
            chunks,
        }
    }

    pub fn run(mut self) -> std::io::Result<()> {
        self.state.within_transaction = true;

        // Make sure that we do not receive any callbacks during the transaction.
        self.disable_callbacks()?;

        loop {
            if !self.send_chunks()? {
                break;
            }

            self.receive()?;
        }

        Ok(())
    }

    /// Sends chunks as long as the retransmit queue is not full.
    fn send_chunks(&mut self) -> std::io::Result<bool> {
        while !self.buffers.retransmits.is_full() {
            if let Some(chunk) = self.chunks.next() {
                self.send_chunk(chunk)?;
            } else {
                return Ok(false);
            }
        }

        Ok(true)
    }

    /// Sends a chunk of data.
    fn send_chunk(&mut self, chunk: &[u8]) -> std::io::Result<()> {
        self.buffers.payload.clear();
        self.buffers
            .payload
            .extend_from_slice(chunk)
            .map_err(|()| {
                Error::new(
                    ErrorKind::OutOfMemory,
                    "ASHv2: could not append chunk to frame buffer",
                )
            })?;
        let data = Data::create(self.state.next_frame_number(), self.buffers.payload.clone());
        self.serial_port
            .write_frame_buffered(&data, &mut self.buffers.frame)
    }

    fn disable_callbacks(&mut self) -> std::io::Result<()> {
        self.ack(self.state.ack_number())
    }

    /// Receives a packet from the serial port.
    ///
    /// Returns `Ok(None)` if no packet was received within the timeout.
    ///
    /// # Errors
    ///
    /// Returns an error if the serial port read operation failed.
    fn receive(&mut self) -> std::io::Result<Option<Packet>> {
        match self
            .serial_port
            .read_packet_buffered(&mut self.buffers.frame)
        {
            Ok(packet) => Ok(Some(packet)),
            Err(error) => {
                if error.kind() == ErrorKind::TimedOut {
                    Ok(None)
                } else {
                    Err(error)
                }
            }
        }
    }

    fn ack(&mut self, ack_number: WrappingU3) -> std::io::Result<()> {
        self.serial_port.write_frame_buffered(
            &Ack::create(ack_number, self.state.n_rdy()),
            &mut self.buffers.frame,
        )
    }

    fn nak(&mut self, ack_number: WrappingU3) -> std::io::Result<()> {
        self.serial_port.write_frame_buffered(
            &Nak::create(ack_number, self.state.n_rdy()),
            &mut self.buffers.frame,
        )
    }

    fn send_data(&mut self, data: Data) -> std::io::Result<()> {
        self.serial_port
            .write_frame_buffered(&data, &mut self.buffers.frame)?;
        self.enqueue_retransmit(data)
    }

    fn enqueue_retransmit(&mut self, data: Data) -> std::io::Result<()> {
        self.buffers
            .retransmits
            .push_front(data.into())
            .map_err(|_| {
                Error::new(
                    ErrorKind::OutOfMemory,
                    "ASHv2: failed to enqueue retransmit",
                )
            })
    }

    fn retransmit(&mut self) -> std::io::Result<bool> {
        for _ in 0..self.buffers.retransmits.len() {
            if let Some(retransmit) = self.buffers.retransmits.pop_back() {
                if retransmit.is_timed_out() {
                    let data = retransmit.into_data();
                    warn!("Retransmitting {:?}", data);
                    self.send_data(data)?;
                    return Ok(true);
                }
            }
        }

        Ok(false)
    }
}
