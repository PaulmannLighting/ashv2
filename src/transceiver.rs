mod buffers;
mod callback_handler;
mod channels;
mod retransmit;
mod state;
mod status;
mod transaction;

use crate::ash_read::AshRead;
use crate::packet::{Data, Packet};
use crate::protocol::AshChunks;
use crate::request::Request;
use buffers::Buffers;
use callback_handler::CallbackHandler;
use channels::Channels;
use log::{debug, error};
use serialport::TTYPort;
use state::State;
use status::Status;
use std::sync::mpsc::{Receiver, Sender};
use std::time::{Duration, SystemTime};
use transaction::Transaction;

const T_REMOTE_NOTRDY: Duration = Duration::from_secs(1);

type PayloadBuffer = heapless::Vec<u8, { Data::MAX_PAYLOAD_SIZE }>;

#[derive(Debug)]
pub struct Transceiver {
    serial_port: TTYPort,
    channels: Channels,
    buffers: Buffers,
    state: State,
}

impl Transceiver {
    #[must_use]
    pub const fn new(
        serial_port: TTYPort,
        requests: Receiver<Request>,
        callback: Option<Sender<Box<[u8]>>>,
    ) -> Self {
        Self {
            serial_port,
            channels: Channels::new(requests, callback),
            buffers: Buffers::new(),
            state: State::new(),
        }
    }

    pub fn run(mut self) {
        loop {
            if let Err(error) = self.main() {
                error!("I/O error: {error}");
                self.state.reject = true;
            }
        }
    }

    fn main(&mut self) -> std::io::Result<()> {
        match self.state.status {
            Status::Disconnected | Status::Failed => self.connect(),
            Status::Connected => self.transceive(),
        }
    }

    pub fn connect(&mut self) -> std::io::Result<()> {
        debug!("Connecting to NCP...");
        let start = SystemTime::now();

        loop {
            self.reset()?;

            if let Packet::RstAck(rst_ack) = self
                .serial_port
                .read_packet_buffered(&mut self.buffers.frame)?
            {
                debug!("Received RSTACK: {rst_ack}");
                self.state.status = Status::Connected;

                if let Ok(elapsed) = start.elapsed() {
                    debug!("Connection established after {elapsed:?}");
                }

                return Ok(());
            }
        }
    }

    pub fn transceive(&mut self) -> std::io::Result<()> {
        if self.state.reject {
            return self.try_clear_reject_condition();
        }

        if let Some(bytes) = self.channels.receive()? {
            Transaction::new(
                &mut self.serial_port,
                &mut self.channels,
                &mut self.buffers,
                &mut self.state,
                bytes.ash_chunks()?,
            )
            .run()?;
        } else {
            CallbackHandler::new(
                &mut self.serial_port,
                &mut self.channels,
                &mut self.buffers,
                &mut self.state,
            )
            .run()?;
        }

        Ok(())
    }

    fn reset(&mut self) -> std::io::Result<()> {
        todo!("Reset connection")
    }

    fn try_clear_reject_condition(&mut self) -> std::io::Result<()> {
        todo!("Clear reject condition")
    }
}
