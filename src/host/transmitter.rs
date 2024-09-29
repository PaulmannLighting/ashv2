use std::collections::HashMap;
use std::fmt::Debug;
use std::slice::Chunks;
use std::sync::atomic::Ordering::SeqCst;
use std::sync::atomic::{AtomicBool, AtomicU8};
use std::sync::mpsc::Receiver;
use std::sync::Arc;
use std::thread::sleep;
use std::time::{Duration, SystemTime};

use log::{debug, error, info, trace};
use serialport::TTYPort;

use crate::ash_write::AshWrite;
use crate::error::{frame, Error};
use crate::frame_buffer::FrameBuffer;
use crate::packet::{Data, Rst};
use crate::protocol::{AshChunks, Command, Event, Handler};
use crate::util::{next_three_bit_number, NonPoisonedRwLock};

const MAX_STARTUP_ATTEMPTS: u8 = 5;
const MAX_TIMEOUTS: usize = 4;
const T_REMOTE_NOTRDY: Duration = Duration::from_millis(1000);
const T_RSTACK_MAX: Duration = Duration::from_millis(3200);
const T_RX_ACK_INIT: Duration = Duration::from_millis(1600);
const T_RX_ACK_MAX: Duration = Duration::from_millis(3200);
const T_RX_ACK_MIN: Duration = Duration::from_millis(400);

#[derive(Debug)]
pub struct Transmitter {
    // Shared state
    serial_port: TTYPort,
    running: Arc<AtomicBool>,
    connected: Arc<AtomicBool>,
    command: Receiver<Command>,
    handler: Arc<NonPoisonedRwLock<Option<Arc<dyn Handler>>>>,
    ack_number: Arc<AtomicU8>,
    ack_receiver: Receiver<u8>,
    nak_receiver: Receiver<u8>,
    // Local state
    buffer: FrameBuffer,
    sent: heapless::Vec<(SystemTime, Data), MAX_TIMEOUTS>,
    retransmit: heapless::Deque<Data, MAX_TIMEOUTS>,
    retransmits: HashMap<u8, usize>,
    frame_number: u8,
    t_rx_ack: Duration,
}

impl Transmitter {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        serial_port: TTYPort,
        running: Arc<AtomicBool>,
        connected: Arc<AtomicBool>,
        command: Receiver<Command>,
        handler: Arc<NonPoisonedRwLock<Option<Arc<dyn Handler>>>>,
        ack_number: Arc<AtomicU8>,
        ack_receiver: Receiver<u8>,
        nak_receiver: Receiver<u8>,
    ) -> Self {
        Self {
            serial_port,
            running,
            connected,
            command,
            handler,
            ack_number,
            ack_receiver,
            nak_receiver,
            buffer: FrameBuffer::new(),
            sent: heapless::Vec::new(),
            retransmit: heapless::Deque::new(),
            retransmits: HashMap::new(),
            frame_number: 0,
            t_rx_ack: T_RX_ACK_INIT,
        }
    }

    pub fn run(mut self) {
        while self.running.load(SeqCst) {
            if let Err(error) = self.main() {
                error!("{error}");
                self.running.store(false, SeqCst);
                break;
            }
        }

        debug!("Terminating.");
    }

    fn main(&mut self) -> Result<(), Error> {
        if self.connected.load(SeqCst) {
            if self.handler.read().is_some() {
                trace!("Waiting for current transaction to complete.");
                Ok(())
            } else {
                trace!("Processing next command.");
                self.process_next_command()
            }
        } else {
            self.initialize()
        }
    }

    fn process_next_command(&mut self) -> Result<(), Error> {
        match self.command.recv() {
            Ok(command) => self.process_command(command),
            Err(error) => {
                error!("Error receiving command: {error}");
                Ok(())
            }
        }
    }

    fn process_command(&mut self, command: Command) -> Result<(), Error> {
        trace!(
            "Processing command {:#04X?} with handler {:#?}",
            &command.payload,
            &command.handler
        );
        self.handler.write().replace(command.handler);
        self.transmit_data(&command.payload)
    }

    fn transmit_data(&mut self, payload: &[u8]) -> Result<(), Error> {
        if let Err(error) = payload
            .ash_chunks()
            .and_then(|chunks| self.transmit_chunks(chunks))
        {
            error!("{error}");
            self.abort_current_transaction(error);
            info!("Re-initializing connection.");
            self.initialize()
        } else {
            debug!("Transmission completed.");
            self.set_transmission_completed();
            Ok(())
        }
    }

    fn transmit_chunks(&mut self, mut chunks: Chunks<'_, u8>) -> Result<(), Error> {
        let mut transmits;

        loop {
            if !self.connected.load(SeqCst) {
                error!("Connection lost during transaction.");
                return Err(Error::Aborted);
            }

            if !self.running.load(SeqCst) {
                error!("Terminated during active transaction.");
                return Err(Error::Terminated);
            }

            self.handle_naks_and_acks();
            transmits = 0;
            transmits += self.retransmit()?;
            transmits += self.push_chunks(&mut chunks)?;

            if transmits == 0 && self.is_transaction_complete() {
                return Ok(());
            }
        }
    }

    fn retransmit(&mut self) -> Result<usize, Error> {
        let mut retransmits: usize = 0;

        while self.sent.len() < MAX_TIMEOUTS {
            if let Some(mut data) = self.retransmit.pop_front() {
                let cnt = self.retransmits.entry(data.frame_num()).or_default();
                *cnt += 1;

                if *cnt > MAX_TIMEOUTS {
                    error!("Max retransmits exceeded for frame #{}", data.frame_num());
                    return Err(Error::MaxRetransmitsExceeded);
                }

                retransmits += 1;
                debug!("Retransmitting: {data}");
                trace!("{data:#04X?}");
                data.set_is_retransmission(true);
                self.send_data(data)
                    .inspect_err(|error| error!("Failed to retransmit: {error}"))?;
            } else {
                break;
            }
        }

        Ok(retransmits)
    }

    fn push_chunks(&mut self, chunks: &mut Chunks<'_, u8>) -> Result<usize, Error> {
        let mut transmits: usize = 0;

        while self.sent.len() < MAX_TIMEOUTS {
            if let Some(chunk) = chunks.next() {
                transmits += 1;
                self.buffer.clear();
                self.buffer.extend_from_slice(chunk).map_err(|()| {
                    std::io::Error::new(
                        std::io::ErrorKind::OutOfMemory,
                        "Buffer should be large enough.",
                    )
                })?;
                self.send_chunk()
                    .inspect_err(|error| error!("Error during transmission of chunk: {error}"))?;
            } else {
                break;
            }
        }

        Ok(transmits)
    }

    fn send_chunk(&mut self) -> Result<(), Error> {
        debug!("Sending chunk.");
        trace!("Buffer: {:#04X?}", &*self.buffer);
        let data = Data::create(
            self.next_frame_number(),
            self.ack_number.load(SeqCst),
            self.buffer.as_slice().try_into().map_err(|()| {
                Error::Frame(frame::Error::PayloadTooLarge {
                    max: Data::MAX_PAYLOAD_SIZE,
                    size: self.buffer.len(),
                })
            })?,
        );
        self.send_data(data)
    }

    fn send_data(&mut self, data: Data) -> Result<(), Error> {
        debug!("Sending data: {data}");
        trace!("{data:#04X?}");

        if self.connected.load(SeqCst) {
            self.serial_port
                .write_frame_buffered(&data, &mut self.buffer)?;
            self.sent.push((SystemTime::now(), data)).map_err(|_| {
                std::io::Error::new(
                    std::io::ErrorKind::OutOfMemory,
                    "failed to push data to sent queue",
                )
            })?;
            Ok(())
        } else {
            error!("Attempted to transmit while not connected.");
            Err(Error::Aborted)
        }
    }

    fn handle_naks_and_acks(&mut self) {
        self.handle_naks();
        self.check_ack_timeouts();
        self.handle_acks();
    }

    fn handle_naks(&mut self) {
        #[allow(clippy::needless_collect)] // Polonius issue.
        for ack_num in self
            .nak_receiver
            .try_iter()
            .collect::<heapless::Vec<u8, MAX_TIMEOUTS>>()
        {
            self.handle_nak(ack_num);
        }
    }

    fn handle_nak(&mut self, nak_num: u8) {
        if let Some((_, data)) = self
            .sent
            .iter()
            .position(|(_, data)| data.frame_num() == nak_num)
            .map(|index| self.sent.remove(index))
        {
            self.retransmit
                .push_back(data)
                .expect("Retransmit queue should always accept data.");
        }
    }

    fn handle_acks(&mut self) {
        #[allow(clippy::needless_collect)] // Polonius issue.
        for ack_num in self
            .ack_receiver
            .try_iter()
            .collect::<heapless::Vec<u8, MAX_TIMEOUTS>>()
        {
            self.handle_ack(ack_num);
        }
    }

    fn handle_ack(&mut self, ack_num: u8) {
        trace!("Handling ACK: {ack_num}");
        if let Some((timestamp, data)) = self
            .sent
            .iter()
            .position(|(_, data)| next_three_bit_number(data.frame_num()) == ack_num)
            .map(|index| self.sent.remove(index))
        {
            trace!("ACKed packet #{}", data.frame_num());
            if let Ok(duration) = SystemTime::now().duration_since(timestamp) {
                self.update_t_rx_ack(Some(duration));
            }
        }
    }

    fn check_ack_timeouts(&mut self) {
        let now = SystemTime::now();

        while let Some((_, data)) = self
            .sent
            .iter()
            .position(|(timestamp, _)| {
                now.duration_since(*timestamp)
                    .map_or(false, |duration| duration > self.t_rx_ack)
            })
            .map(|index| self.sent.remove(index))
        {
            self.retransmit
                .push_back(data)
                .expect("Retransmit queue should always accept data.");
            self.update_t_rx_ack(None);
        }
    }

    fn update_t_rx_ack(&mut self, last_ack_duration: Option<Duration>) {
        self.t_rx_ack = last_ack_duration
            .map_or_else(
                || self.t_rx_ack * 2,
                |duration| self.t_rx_ack * 7 / 8 + duration / 2,
            )
            .clamp(T_RX_ACK_MIN, T_RX_ACK_MAX);
    }

    fn next_frame_number(&mut self) -> u8 {
        let frame_number = self.frame_number;
        self.frame_number = next_three_bit_number(frame_number);
        frame_number
    }

    fn initialize(&mut self) -> Result<(), Error> {
        let mut sent_rst_timestamp: SystemTime;

        for attempt in 1..=MAX_STARTUP_ATTEMPTS {
            debug!("Establishing ASH connection. Attempt #{attempt}");
            self.reset();
            sent_rst_timestamp = SystemTime::now();

            debug!("Waiting for NCP to start up.");
            while !self.connected.load(SeqCst) {
                trace!("Waiting for NCP to become ready.");
                sleep(T_REMOTE_NOTRDY);

                match SystemTime::now().duration_since(sent_rst_timestamp) {
                    Ok(duration) => {
                        trace!("Time passed: {duration:?}");
                        if duration > T_RSTACK_MAX {
                            break;
                        }
                    }
                    Err(error) => {
                        error!("System time jumped: {error}");
                        sent_rst_timestamp = SystemTime::now();
                    }
                }
            }

            if self.connected.load(SeqCst) {
                debug!("ASH connection established.");
                return Ok(());
            }
        }

        error!("Failed to establish ASH connection.");
        Err(Error::InitializationFailed)
    }

    fn reset(&mut self) {
        debug!("Resetting connection.");
        self.connected.store(false, SeqCst);
        trace!("Sending RST.");
        self.serial_port
            .write_frame_buffered(&Rst::default(), &mut self.buffer)
            .unwrap_or_else(|error| error!("Failed to send RST: {error}"));
        self.reset_state();
    }

    fn reset_state(&mut self) {
        debug!("Resetting state.");
        self.abort_current_transaction(Error::Aborted);
        self.buffer.clear();
        self.sent.clear();
        self.retransmits.clear();
        self.retransmit.clear();
        self.frame_number = 0;
        self.ack_number.store(0, SeqCst);
        self.t_rx_ack = T_RX_ACK_INIT;
    }

    fn abort_current_transaction(&self, error: Error) {
        let handler = self.handler.write().take();

        if let Some(handler) = handler {
            handler.abort(error);
            handler.wake();
        }
    }

    fn set_transmission_completed(&self) {
        if let Some(handler) = self.handler.read().as_ref() {
            debug!("Finalizing data command.");
            handler.handle(Event::TransmissionCompleted);
        }
    }

    fn is_transaction_complete(&self) -> bool {
        self.sent.is_empty() && self.retransmit.is_empty()
    }
}
