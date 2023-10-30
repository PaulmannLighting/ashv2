mod buffers;
mod state;

use super::Transaction;
use crate::frame::Frame;
use crate::packet::{Ack, Data, Error, Nak, Packet, Rst, RstAck};
use crate::protocol::{AshChunks, Mask, Stuffing, CANCEL, FLAG, SUBSTITUTE, TIMEOUT, X_OFF, X_ON};
use buffers::Buffers;
use itertools::{Chunk, Itertools};
use log::{debug, error, info, trace, warn};
use serialport::SerialPort;
use state::State;
use std::fmt::{Debug, Display};
use std::iter::Copied;
use std::slice::Iter;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::Receiver;
use std::sync::Arc;
use std::thread::sleep;
use std::time::Duration;

const MAX_STARTUP_ATTEMPTS: u8 = 5;
const T_RSTACK_MAX: Duration = Duration::from_millis(3200);
const T_TX_ACK_DELAY: Duration = Duration::from_millis(20);
const T_REMOTE_NOTRDY: Duration = Duration::from_millis(1000);

type Chunks<'c, 'i> = Vec<Chunk<'c, Copied<Iter<'i, u8>>>>;

#[derive(Debug)]
pub struct Worker<S>
where
    S: SerialPort,
{
    // Shared state
    receiver: Receiver<Transaction>,
    terminate: Arc<AtomicBool>,
    // Local state
    serial_port: S,
    state: State,
    buffers: Buffers,
}

impl<S> Worker<S>
where
    S: SerialPort,
{
    pub fn new(
        serial_port: S,
        receiver: Receiver<Transaction>,
        terminate: Arc<AtomicBool>,
    ) -> Self {
        Self {
            receiver,
            terminate,
            serial_port,
            state: State::default(),
            buffers: Buffers::default(),
        }
    }

    pub fn spawn(mut self) {
        while !self.terminate.load(Ordering::SeqCst) {
            debug!("Waiting for next request.");
            match self.receiver.recv() {
                Ok(transaction) => self.process_transaction(transaction),
                Err(error) => error!("{error}"),
            }
        }
    }

    fn process_transaction(&mut self, transaction: Transaction) {
        trace!("Processing transaction: {:#04X?}", transaction);

        if !self.state.initialized() {
            debug!("NCP not initialized.");
            match self.initialize() {
                Ok(_) => self.state.set_initialized(),
                Err(error) => {
                    self.terminate.store(true, Ordering::SeqCst);
                    transaction.resolve_error(error);
                    return;
                }
            }
        }

        match transaction {
            Transaction::Data(future) => {
                let result = self.process_data_request(future.request());
                future.resolve(result);
            }
            Transaction::Reset(future) => future.resolve(self.reset()),
            Transaction::Terminate => (),
        }
    }

    fn process_data_request(&mut self, data: &Arc<[u8]>) -> Result<Arc<[u8]>, crate::Error> {
        self.buffers.clear();
        let result = data
            .iter()
            .copied()
            .ash_chunks()
            .and_then(|chunks| self.process_chunks(chunks.into_iter().collect_vec()));

        trace!("Transaction result: {result:#04X?}");

        if let Err(error) = &result {
            self.recover_error(error);
        }

        result
    }

    fn process_chunks(&mut self, mut chunks: Chunks) -> Result<Arc<[u8]>, crate::Error> {
        while !self.terminate.load(Ordering::SeqCst) {
            debug!("Processing chunk...");
            if self
                .buffers
                .output
                .queue_retransmit_timeout(self.state.t_rx_ack())
            {
                self.state.update_t_rx_ack(None);
            }

            if self.state.may_transmit() {
                self.retransmit()?;
                self.push_chunks(&mut chunks)?;
            }

            self.receive_and_process_packet()?;
            sleep(T_TX_ACK_DELAY);

            if self.state.is_rejecting() {
                debug!("Reject condition is active. Sending NAK.");
                self.send_nak()?;
                continue;
            }

            self.send_pending_acks()?;

            if self.is_transaction_complete(&chunks) {
                return Ok(self.buffers.input.bytes());
            }
        }

        Err(crate::Error::Terminated)
    }

    fn receive_and_process_packet(&mut self) -> Result<(), crate::Error> {
        debug!("Receiving packet.");

        match self.receive_packet() {
            Err(error) => self.state.handle_error(error),
            Ok(packet) => self.process_packet(packet),
        }
    }

    fn process_packet(&mut self, packet: Packet) -> Result<(), crate::Error> {
        if packet.is_crc_valid() {
            match packet {
                Packet::Ack(ref ack) => self.process_ack(ack),
                Packet::Data(data) => self.process_data(data)?,
                Packet::Error(ref error) => self.handle_error(error)?,
                Packet::Nak(ref nak) => self.process_nak(nak),
                Packet::Rst(ref rst) => {
                    error!("NCP sent us an unexpected RST: {rst}");
                    trace!("Frame details: {rst:#04X?}");
                }
                Packet::RstAck(ref rst_ack) => self.process_rst_ack(rst_ack),
            }
        } else {
            warn!("Received frame with invalid CRC: {packet}");
            trace!("Frame details: {packet:#04X?}");
            self.send_nak()?;
        }

        Ok(())
    }

    fn process_ack(&mut self, ack: &Ack) {
        debug!("Received frame: {ack}");
        trace!("Frame details: {ack:#04X?}");
        self.ack_sent_data(ack.ack_num());
    }

    fn process_data(&mut self, data: Data) -> Result<(), crate::Error> {
        debug!("Received frame: {data}");
        trace!("Frame details: {data:#04X?}");
        trace!(
            "Unmasked payload: {:#04X?}",
            data.payload().iter().copied().mask().collect_vec()
        );

        if data.frame_num() == self.state.ack_number() {
            self.state.set_rejecting(false);
            self.state.set_last_received_frame_number(data.frame_num());
            debug!("Last received frame number: {}", data.frame_num());
            self.ack_sent_data(data.ack_num());
            self.buffers.input.push_data(data);
        } else if data.is_retransmission() {
            self.ack_sent_data(data.ack_num());
            self.buffers.input.push_data(data);
        } else {
            debug!("Received out-of-sequence data frame: {data}");

            if !self.state.is_rejecting() {
                self.reject()?;
            }
        }

        Ok(())
    }

    fn handle_error(&mut self, error: &Error) -> Result<(), crate::Error> {
        debug!("Received frame: {error}");
        trace!("Frame details: {error:#04X?}");

        error.code().map_or_else(
            || {
                error!("NCP set error without valid code.");
                trace!("NCP error message was: {error}");
            },
            |code| {
                error!("NCP sent error condition: {code}");
            },
        );

        self.reset()
    }

    fn process_nak(&mut self, nak: &Nak) {
        debug!("Received frame: {nak}");
        trace!("Frame details: {nak:#04X?}");
        self.buffers.output.queue_retransmit_nak(nak.ack_num());
    }

    fn process_rst_ack(&mut self, rst_ack: &RstAck) {
        debug!("Received frame: {rst_ack}");
        trace!("Frame details: {rst_ack:#04X?}");
        self.state.set_may_transmit(true);
        rst_ack.code().map_or_else(
            || {
                error!("NCP acknowledged reset with invalid error code.");
                trace!("NCP response was: {rst_ack}");
            },
            |code| {
                debug!("NCP acknowledged reset due to: {code}");
            },
        );
    }

    fn retransmit(&mut self) -> std::io::Result<()> {
        while self.buffers.output.queue_not_full() {
            if let Some(mut data) = self.buffers.output.pop_retransmit() {
                data.set_is_retransmission(true);
                debug!("Retransmitting data frame: {data}");
                trace!("Frame details: {data:#04X?}");
                self.send_data(data)?;
            } else {
                return Ok(());
            }
        }

        debug!("No transmission slots free.");
        Ok(())
    }

    fn push_chunks(&mut self, chunks: &mut Chunks) -> Result<(), crate::Error> {
        while self.buffers.output.queue_not_full() {
            if let Some(chunk) = chunks.pop() {
                debug!("Transmitting chunk.");
                let data =
                    Data::try_from((self.state.next_frame_number(), chunk.collect_vec().into()))?;
                self.send_data(data)?;
            } else {
                debug!("No more chunks to transmit.");
                return Ok(());
            }
        }

        debug!("No transmission slots free.");
        Ok(())
    }

    fn send_pending_acks(&mut self) -> std::io::Result<()> {
        for ack_number in self.state.pending_acks() {
            self.send_ack(ack_number)?;
        }

        Ok(())
    }

    fn reject(&mut self) -> std::io::Result<()> {
        self.state.set_rejecting(true);
        self.send_nak()
    }

    fn send_ack(&mut self, ack_number: u8) -> std::io::Result<()> {
        self.send_frame(&Ack::from_ack_num(ack_number))?;
        self.state.set_last_sent_ack(ack_number);
        Ok(())
    }

    fn send_data(&mut self, data: Data) -> std::io::Result<()> {
        trace!("Sending data frame with payload: {:#04X?}", data.payload());
        trace!(
            "Sending data frame with unmasked payload: {:#04X?}",
            data.payload().iter().copied().mask().collect_vec()
        );
        self.send_frame(&data)?;
        self.buffers.output.push_data(data);
        Ok(())
    }

    fn send_nak(&mut self) -> std::io::Result<()> {
        self.state.last_received_frame_number().map_or_else(
            || {
                error!("No frame received yet. Nothing to reject.");
                Ok(())
            },
            |last_received_frame_number| {
                self.send_frame(&Nak::from_ack_num(last_received_frame_number))
            },
        )
    }

    fn send_frame<F>(&mut self, frame: F) -> std::io::Result<()>
    where
        F: Debug + Display + IntoIterator<Item = u8>,
    {
        debug!("Sending frame: {frame}");
        trace!("Frame details: {frame:#04X?}");
        self.serial_port
            .write_all(self.buffers.output.buffer_frame(frame.into_iter().stuff()))
    }

    fn ack_sent_data(&mut self, ack_num: u8) {
        debug!("Acknowledged frame: {ack_num}");

        if let Some(duration) = self.buffers.output.last_ack_duration(ack_num) {
            debug!("Last ACK duration: {} sec", duration.as_secs_f32());
            self.state.update_t_rx_ack(Some(duration));
            debug!("New T_RX_ACK: {} sec", self.state.t_rx_ack().as_secs_f32());
        }

        self.buffers.output.ack_sent_data(ack_num);
    }

    fn receive_packet(&mut self) -> Result<Packet, crate::Error> {
        Ok(Packet::try_from(self.receive_frame()?.as_slice())?)
    }

    fn receive_frame(&mut self) -> Result<Vec<u8>, crate::Error> {
        self.buffers.input.buffer_mut().clear();
        let mut error = false;

        while !self.terminate.load(Ordering::SeqCst) {
            match self.buffers.input.read_byte(&mut self.serial_port)? {
                CANCEL => {
                    self.buffers.input.buffer_mut().clear();
                    error = false;
                }
                FLAG => {
                    if !error && !self.buffers.input.buffer_mut().is_empty() {
                        debug!("Received frame.");
                        trace!("Frame details: {:#04X?}", self.buffers.input.buffer_mut());
                        return Ok(self.buffers.input.frame_bytes());
                    }

                    self.buffers.input.buffer_mut().clear();
                    error = false;
                }
                SUBSTITUTE => {
                    error = true;
                }
                X_ON => {
                    info!("NCP requested to stop transmission.");
                    self.state.set_may_transmit(true);
                }
                X_OFF => {
                    info!("NCP requested to resume transmission.");
                    self.state.set_may_transmit(false);
                }
                TIMEOUT => {
                    warn!("Received timeout byte not specified in protocol definition.");
                }
                byte => self.buffers.input.buffer_mut().push(byte),
            }
        }

        Err(crate::Error::Terminated)
    }

    fn recover_error(&mut self, error: &crate::Error) {
        match error {
            crate::Error::Io(error) => {
                debug!("Attempting to recover from I/O error: {error}");

                if let Err(error) = self.reset() {
                    error!("Failed to reset connection: {error}");
                }
            }
            error => error!("Recovering from this error type is not implemented: {error}"),
        }
    }

    fn reset(&mut self) -> Result<(), crate::Error> {
        self.serial_port.set_timeout(T_RSTACK_MAX)?;
        self.send_frame(&Rst::default())?;

        loop {
            match self.receive_packet()? {
                Packet::RstAck(ref rst_ack) => {
                    self.state.reset();
                    self.process_rst_ack(rst_ack);
                    return Ok(());
                }
                packet => trace!("Ignoring packet: {packet}."),
            }
        }
    }

    fn initialize(&mut self) -> Result<(), crate::Error> {
        for attempt in 1..=MAX_STARTUP_ATTEMPTS {
            match self.reset() {
                Ok(_) => {
                    debug!("ASH connection initialized after {attempt} attempts.");
                    return Ok(());
                }
                Err(error) => {
                    warn!("Startup attempt #{attempt} failed.");
                    debug!("{error}");
                }
            }

            sleep(T_REMOTE_NOTRDY);
        }

        error!("Startup failed after {MAX_STARTUP_ATTEMPTS} tries.");
        Err(crate::Error::InitializationFailed)
    }

    fn is_transaction_complete(&self, chunks: &Chunks) -> bool {
        trace!("Chunks empty: {}", chunks.is_empty());
        trace!("No pending ACKs: {}", self.state.pending_acks().is_empty());
        chunks.is_empty()
            && self.state.pending_acks().is_empty()
            && self.buffers.output.queues_are_empty()
    }
}
