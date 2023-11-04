use crate::frame::Frame;
use crate::packet::{Data, Rst};
use crate::protocol::host::command::{Command, Event, Response};
use crate::protocol::AshChunks;
use crate::util::next_three_bit_number;
use crate::{AshWrite, Error};
use itertools::Chunks;
use log::{debug, error, info, trace};
use serialport::SerialPort;
use std::collections::VecDeque;
use std::fmt::Debug;
use std::iter::Copied;
use std::slice::Iter;
use std::sync::atomic::Ordering::SeqCst;
use std::sync::atomic::{AtomicBool, AtomicU8};
use std::sync::mpsc::Receiver;
use std::sync::{Arc, Mutex, MutexGuard};
use std::thread::sleep;
use std::time::{Duration, SystemTime};

const MAX_STARTUP_ATTEMPTS: u8 = 5;
const MAX_TIMEOUTS: usize = 4;
const T_REMOTE_NOTRDY: Duration = Duration::from_millis(1000);
const T_RSTACK_MAX: Duration = Duration::from_millis(3200);
const T_RX_ACK_INIT: Duration = Duration::from_millis(1600);
const T_RX_ACK_MAX: Duration = Duration::from_millis(3200);
const T_RX_ACK_MIN: Duration = Duration::from_millis(400);

#[derive(Debug)]
pub struct Transmitter<S>
where
    S: SerialPort,
{
    // Shared state
    serial_port: Arc<Mutex<S>>,
    running: Arc<AtomicBool>,
    connected: Arc<AtomicBool>,
    command: Receiver<Command>,
    current_command: Arc<Mutex<Option<Command>>>,
    ack_number: Arc<AtomicU8>,
    ack_receiver: Receiver<u8>,
    nak_receiver: Receiver<u8>,
    // Local state
    buffer: Vec<u8>,
    sent: Vec<(SystemTime, Data)>,
    retransmit: VecDeque<Data>,
    frame_number: u8,
    t_rx_ack: Duration,
}

impl<S> Transmitter<S>
where
    S: SerialPort,
{
    pub fn new(
        serial_port: Arc<Mutex<S>>,
        running: Arc<AtomicBool>,
        connected: Arc<AtomicBool>,
        command: Receiver<Command>,
        current_command: Arc<Mutex<Option<Command>>>,
        ack_number: Arc<AtomicU8>,
        ack_receiver: Receiver<u8>,
        nak_receiver: Receiver<u8>,
    ) -> Self {
        Self {
            serial_port,
            running,
            connected,
            command,
            current_command,
            ack_number,
            ack_receiver,
            nak_receiver,
            buffer: Vec::new(),
            sent: Vec::new(),
            retransmit: VecDeque::new(),
            frame_number: 0,
            t_rx_ack: T_RX_ACK_INIT,
        }
    }

    pub fn spawn(mut self) {
        while self.running.load(SeqCst) {
            self.main();
        }

        info!("Terminating.");
    }

    fn main(&mut self) {
        if self.connected.load(SeqCst) {
            if self.current_command().as_ref().is_some() {
                trace!("Waiting for current transaction to complete.");
            } else {
                self.process_next_command();
            }
        } else {
            self.initialize();
        }
    }

    fn process_next_command(&mut self) {
        match self.command.recv() {
            Ok(command) => self.process_command(command),
            Err(error) => error!("Error receiving command: {error}"),
        }
    }

    fn process_command(&mut self, command: Command) {
        self.current_command().replace(command);
        let command_clone = self.current_command().as_ref().cloned();

        if let Some(command) = command_clone {
            match command {
                Command::Data(payload, _) => self.transmit_data(&payload),
                Command::Reset(_) => self.reset(),
                Command::Terminate => self.running.store(false, SeqCst),
            };
        }
    }

    fn transmit_data(&mut self, payload: &[u8]) {
        if let Err(error) = payload
            .iter()
            .copied()
            .ash_chunks()
            .and_then(|chunks| self.transmit_chunks(chunks.into_iter()))
        {
            error!("{error}");
            self.abort_current_command(error);
        } else {
            info!("Transmission completed.");
            self.complete_current_command();
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
                retransmits += 1;
                info!("Retransmitting: {data}");
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
        trace!("{:#04X?}", self.buffer);
        let mut data = Data::try_from((self.next_frame_number(), self.buffer.as_ref()))?;
        data.set_ack_num(self.ack_number.load(SeqCst));
        self.send_data(data)
    }

    fn send_data(&mut self, data: Data) -> Result<(), Error> {
        debug!("Sending data: {data}");
        trace!("{data:#04X?}");

        if self.connected.load(SeqCst) {
            self.write_frame(&data)?;
            self.sent.push((SystemTime::now(), data));
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
        self.buffer.clear();
        self.buffer.extend(self.nak_receiver.try_iter());

        // Hack around non-Polonius issue.
        let mut nak_num;
        for index in 0..self.buffer.len() {
            nak_num = unsafe { self.buffer.get_unchecked(index) };
            self.handle_nak(*nak_num);
        }
    }

    fn handle_nak(&mut self, nak_num: u8) {
        if let Some((_, data)) = self
            .sent
            .iter()
            .position(|(_, data)| data.frame_num() == nak_num)
            .map(|index| self.sent.remove(index))
        {
            self.retransmit.push_back(data);
        }
    }

    fn handle_acks(&mut self) {
        self.buffer.clear();
        self.buffer.extend(self.ack_receiver.try_iter());

        // Hack around non-Polonius issue.
        let mut ack_num;
        for index in 0..self.buffer.len() {
            ack_num = unsafe { self.buffer.get_unchecked(index) };
            self.handle_ack(*ack_num);
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
            self.retransmit.push_back(data);
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
        frame_number
    }

    fn initialize(&mut self) {
        let mut sent_rst_timestamp: SystemTime;

        for attempt in 1..=MAX_STARTUP_ATTEMPTS {
            info!("Establishing ASH connection. Attempt #{attempt}");
            self.reset();
            info!("Connection reset.");
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
                info!("ASH connection established.");
                return;
            }
        }

        error!("Failed to establish ASH connection.");
    }

    fn reset(&mut self) {
        info!("Resetting connection.");
        self.connected.store(false, SeqCst);
        debug!("Resetting state.");
        self.reset_state();
        debug!("Sending RST.");
        self.write_frame(&Rst::default())
            .unwrap_or_else(|error| error!("Failed to send RST: {error}"));
    }

    fn reset_state(&mut self) {
        debug!("Aborting current command.");
        self.abort_current_command(Error::Aborted);
        debug!("Cleaning buffer.");
        self.buffer.clear();
        debug!("Clearing sent queue.");
        self.sent.clear();
        debug!("Resetting frame number.");
        self.frame_number = 0;
        debug!("Resetting T_RX_ACK.");
        self.t_rx_ack = T_RX_ACK_INIT;
    }

    fn abort_current_command(&mut self, error: Error) {
        if let Some(current_command) = self.current_command().take() {
            match current_command {
                Command::Data(_, response) => response.abort(error),
                Command::Reset(response) => response.abort(error),
                Command::Terminate => (),
            };
        }
    }

    fn complete_current_command(&mut self) {
        if let Some(current_command) = self.current_command().take() {
            match current_command {
                Command::Data(_, response) => {
                    response.handle(Event::TransmissionCompleted);
                }
                Command::Reset(response) => {
                    response.handle(Event::TransmissionCompleted);
                }
                Command::Terminate => (),
            };
        }
    }

    fn write_frame<F>(&mut self, frame: &F) -> std::io::Result<()>
    where
        F: Frame,
        for<'a> &'a F: IntoIterator<Item = u8>,
    {
        self.serial_port
            .lock()
            .map_err(|error| error!("Failed to lock serial port: {error}"))
            .expect("Failed to lock serial port.")
            .write_frame(frame, &mut self.buffer)
    }

    fn current_command(&self) -> MutexGuard<'_, Option<Command>> {
        self.current_command
            .lock()
            .map_err(|error| error!("Could not lock current command: {error}"))
            .expect("Could not lock current command.")
    }

    fn is_transaction_complete(&self) -> bool {
        self.sent.is_empty() && self.retransmit.is_empty()
    }
}
