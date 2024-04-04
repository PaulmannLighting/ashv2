use std::collections::HashMap;
use std::fmt::Debug;
use std::iter::Copied;
use std::slice::Iter;
use std::sync::atomic::Ordering::SeqCst;
use std::sync::atomic::{AtomicBool, AtomicU8};
use std::sync::mpsc::Receiver;
use std::sync::{Arc, Mutex};
use std::thread::sleep;
use std::time::{Duration, SystemTime};

use itertools::Chunks;
use log::{debug, error, info, trace};
use serialport::SerialPort;

use crate::error::frame;
use crate::frame::Frame;
use crate::packet::{Data, FrameBuffer, Rst, MAX_PAYLOAD_SIZE};
use crate::protocol::{AshChunks, Command, Event, Handler};
use crate::util::{next_three_bit_number, NonPoisonedRwLock};
use crate::{AshWrite, Error};

const MAX_STARTUP_ATTEMPTS: u8 = 5;
const MAX_TIMEOUTS: usize = 4;
const T_REMOTE_NOTRDY: Duration = Duration::from_millis(1000);
const T_RSTACK_MAX: Duration = Duration::from_millis(3200);
const T_RX_ACK_INIT: Duration = Duration::from_millis(1600);
const T_RX_ACK_MAX: Duration = Duration::from_millis(3200);
const T_RX_ACK_MIN: Duration = Duration::from_millis(400);

#[derive(Debug)]
pub struct Transmitter<'cmd, S>
where
    S: SerialPort,
{
    // Shared state
    serial_port: Arc<Mutex<S>>,
    running: Arc<AtomicBool>,
    connected: Arc<AtomicBool>,
    command: Receiver<Command<'cmd>>,
    handler: Arc<NonPoisonedRwLock<Option<Arc<dyn Handler + 'cmd>>>>,
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

impl<'cmd, S> Transmitter<'cmd, S>
where
    S: SerialPort,
{
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        serial_port: Arc<Mutex<S>>,
        running: Arc<AtomicBool>,
        connected: Arc<AtomicBool>,
        command: Receiver<Command<'cmd>>,
        handler: Arc<NonPoisonedRwLock<Option<Arc<dyn Handler + 'cmd>>>>,
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

    fn process_command(&mut self, command: Command<'cmd>) -> Result<(), Error> {
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
            .iter()
            .copied()
            .ash_chunks()
            .and_then(|chunks| self.transmit_chunks(chunks.into_iter()))
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

    fn transmit_chunks(&mut self, mut chunks: Chunks<Copied<Iter<u8>>>) -> Result<(), Error> {
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

                if let Err(error) = self.send_data(data) {
                    error!("Failed to retransmit: {error}");
                    return Err(error);
                }
            } else {
                break;
            }
        }

        Ok(retransmits)
    }

    fn push_chunks(&mut self, chunks: &mut Chunks<Copied<Iter<u8>>>) -> Result<usize, Error> {
        let mut transmits: usize = 0;

        while self.sent.len() < MAX_TIMEOUTS {
            if let Some(chunk) = chunks.next() {
                transmits += 1;
                self.buffer.clear();
                self.buffer.extend(chunk);

                if let Err(error) = self.send_chunk() {
                    error!("Error during transmission of chunk: {error}");
                    return Err(error);
                }
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
                    max: MAX_PAYLOAD_SIZE,
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
            self.write_frame(&data)?;
            self.sent
                .push((SystemTime::now(), data))
                .expect("Send queue should always accept data.");
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
        debug!("Handling ACK: {ack_num}");
        if let Some((timestamp, data)) = self
            .sent
            .iter()
            .position(|(_, data)| next_three_bit_number(data.frame_num()) == ack_num)
            .map(|index| self.sent.remove(index))
        {
            debug!("ACKed packet #{}", data.frame_num());
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
        self.t_rx_ack = if let Some(duration) = last_ack_duration {
            self.t_rx_ack * 7 / 8 + duration / 2
        } else {
            self.t_rx_ack * 2
        }
        .clamp(T_RX_ACK_MIN, T_RX_ACK_MAX);
    }

    fn next_frame_number(&mut self) -> u8 {
        let frame_number = self.frame_number;
        self.frame_number = next_three_bit_number(frame_number);
        trace!("Returning frame number: {frame_number}");
        frame_number
    }

    fn initialize(&mut self) -> Result<(), Error> {
        let mut sent_rst_timestamp: SystemTime;

        for attempt in 1..=MAX_STARTUP_ATTEMPTS {
            debug!("Establishing ASH connection. Attempt #{attempt}");
            self.reset();
            debug!("Connection reset.");
            sent_rst_timestamp = SystemTime::now();

            debug!("Waiting for NCP to start up.");
            while !self.connected.load(SeqCst) {
                debug!("Waiting for NCP to become ready.");
                sleep(T_REMOTE_NOTRDY);

                match SystemTime::now().duration_since(sent_rst_timestamp) {
                    Ok(duration) => {
                        trace!("Time passed: {duration:?}");
                        if duration > T_RSTACK_MAX {
                            break;
                        }
                    }
                    Err(error) => error!("System time jumped: {error}"),
                }
            }

            debug!("Checking whether NCP has started.");
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
        debug!("Sending RST.");
        self.write_frame(&Rst::default())
            .unwrap_or_else(|error| error!("Failed to send RST: {error}"));
        debug!("Resetting state.");
        self.reset_state();
    }

    fn reset_state(&mut self) {
        debug!("Resetting current state.");
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
        if let Some(handler) = self.handler.write().take() {
            handler.abort(error);
            handler.wake();
        }
    }

    fn set_transmission_completed(&mut self) {
        if let Some(handler) = self.handler.read().clone() {
            debug!("Finalizing data command.");
            handler.handle(Event::TransmissionCompleted);
        }
    }

    fn write_frame<F>(&mut self, frame: &F) -> std::io::Result<()>
    where
        F: Frame,
        for<'f> &'f F: IntoIterator<Item = u8>,
    {
        self.serial_port
            .lock()
            .expect("Serial port should never be poisoned.")
            .write_frame(frame)
    }

    fn is_transaction_complete(&self) -> bool {
        self.sent.is_empty() && self.retransmit.is_empty()
    }
}
