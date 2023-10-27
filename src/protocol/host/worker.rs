use crate::frame::Frame;
use crate::packet::{Ack, Data, Error, Nak, Packet, Rst, RstAck};
use crate::protocol::{
    AshChunks, Mask, Request, Stuffing, Transaction, CANCEL, FLAG, SUBSTITUTE, TIMEOUT, X_OFF, X_ON,
};
use crate::util::Extract;
use itertools::{Chunk, Itertools};
use log::{debug, error, info, trace, warn};
use serialport::SerialPort;
use std::collections::VecDeque;
use std::fmt::{Debug, Display};
use std::iter::Copied;
use std::ops::RangeInclusive;
use std::slice::Iter;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::Receiver;
use std::sync::Arc;
use std::thread::sleep;
use std::time::{Duration, SystemTime};

const ACK_TIMEOUTS: usize = 4;
const MAX_STARTUP_ATTEMPTS: u8 = 5;
const T_RX_ACK_INIT: Duration = Duration::from_millis(1600);
const T_RX_ACK_MIN: Duration = Duration::from_millis(400);
const T_RX_ACK_MAX: Duration = Duration::from_millis(3200);
const T_RSTACK_MAX: Duration = Duration::from_millis(3200);
const T_TX_ACK_DELAY: Duration = Duration::from_millis(20);
const T_REMOTE_NOTRDY: Duration = Duration::from_millis(1000);
const INITIAL_BUFFER_CAPACITY: usize = 220;

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
    initialized: bool,
    frame_number: u8,
    last_received_frame_number: Option<u8>,
    last_sent_ack: u8,
    reject: bool,
    transmit: bool,
    sent_data: Vec<(SystemTime, Data)>,
    retransmit: VecDeque<Data>,
    received_data: Vec<(SystemTime, Data)>,
    receive_buffer: Vec<u8>,
    byte_buffer: [u8; 1],
    send_buffer: Vec<u8>,
    t_rx_ack: Duration,
}

impl<S> Worker<S>
where
    S: SerialPort,
{
    #[must_use]
    pub fn new(
        serial_port: S,
        receiver: Receiver<Transaction>,
        terminate: Arc<AtomicBool>,
    ) -> Self {
        Self {
            receiver,
            terminate,
            serial_port,
            initialized: false,
            frame_number: 0,
            last_received_frame_number: None,
            last_sent_ack: 0,
            reject: false,
            transmit: true,
            sent_data: Vec::new(),
            retransmit: VecDeque::new(),
            received_data: Vec::new(),
            receive_buffer: Vec::with_capacity(INITIAL_BUFFER_CAPACITY),
            byte_buffer: [0],
            send_buffer: Vec::with_capacity(INITIAL_BUFFER_CAPACITY),
            t_rx_ack: T_RX_ACK_INIT,
        }
    }

    pub fn spawn(mut self) {
        while !self.terminate.load(Ordering::SeqCst) {
            debug!("Waiting for next request.");
            match self.receiver.recv() {
                Ok(mut transaction) => self.process_transaction(&mut transaction),
                Err(error) => error!("{error}"),
            }
        }
    }

    fn process_transaction(&mut self, transaction: &mut Transaction) {
        trace!(
            "Processing transaction request: {:#04X?}",
            transaction.request()
        );

        if !self.initialized {
            match self.initialize() {
                Ok(_) => self.initialized = true,
                Err(error) => {
                    self.terminate.store(true, Ordering::SeqCst);
                    transaction.resolve(Err(error));
                    return;
                }
            }
        }

        match transaction.request() {
            Request::Data(data) => transaction.resolve(self.process_data_request(data)),
            Request::Reset => transaction.resolve(self.reset().map(|_| Vec::new().into())),
            Request::Terminate => (),
        }
    }

    fn process_data_request(&mut self, data: &Arc<[u8]>) -> Result<Arc<[u8]>, crate::Error> {
        self.clear_buffers();
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
            self.reevaluate_retransmits();

            if self.transmit {
                self.retransmit()?;
                self.push_chunks(&mut chunks)?;
            }

            self.receive_and_process_packet()?;
            sleep(T_TX_ACK_DELAY);

            if self.reject {
                debug!("Reject condition is active. Sending NAK.");
                self.send_nak()?;
                continue;
            }

            self.send_pending_acks()?;

            if self.is_transaction_complete(&chunks) {
                return Ok(self.received_bytes());
            }
        }

        Err(crate::Error::Terminated)
    }

    fn receive_and_process_packet(&mut self) -> Result<(), crate::Error> {
        debug!("Receiving packet.");

        match self.receive_packet()? {
            Packet::Ack(ref ack) => self.process_ack(ack),
            Packet::Data(data) => self.process_data(data)?,
            Packet::Error(ref error) => self.handle_error(error)?,
            Packet::Nak(ref nak) => self.process_nak(nak),
            Packet::Rst(ref rst) => {
                error!("NCP sent us an unexpected RST: {rst}");
                trace!("Frame details: {rst:#04X?}");
            }
            Packet::RstAck(ref rst_ack) => process_rst_ack(rst_ack),
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

        if !data.is_valid() {
            debug!("Received invalid data. Rejecting.");
            return Ok(self.reject()?);
        }

        if data.frame_num() == self.ack_number() {
            self.reject = false;
            self.last_received_frame_number = Some(data.frame_num());
            debug!("Last received frame number: {}", data.frame_num());
            self.ack_sent_data(data.ack_num());
            self.received_data.push((SystemTime::now(), data));
        } else if data.is_retransmission() {
            self.ack_sent_data(data.ack_num());
            self.received_data.push((SystemTime::now(), data));
        } else {
            debug!("Received out-of-sequence data frame: {data}");

            if !self.reject {
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

        for (_, data) in self
            .sent_data
            .extract(|(_, data)| data.frame_num() >= nak.ack_num())
            .into_iter()
            .sorted_by_key(|(_, data)| data.frame_num())
        {
            debug!("Queueing for retransmit: {data}");
            trace!("Frame details: {data:#04X?}");
            self.retransmit.push_back(data);
        }
    }

    fn reevaluate_retransmits(&mut self) {
        let now = SystemTime::now();

        for (_, data) in self.sent_data.extract(|(timestamp, _)| {
            now.duration_since(*timestamp)
                .map_or(false, |duration| duration > self.t_rx_ack)
        }) {
            warn!("Frame {data} has not been acked in time. Queueing for retransmit.");
            trace!("Frame details: {data:#04X?}");
            self.retransmit.push_back(data);
        }
    }

    // See: 5.6 DATA frame Acknowledgement timing
    fn update_t_rx_ack(&mut self, last_ack_duration: Option<Duration>) {
        self.t_rx_ack = if let Some(duration) = last_ack_duration {
            self.t_rx_ack * 7 / 8 + duration / 2
        } else {
            self.t_rx_ack * 2
        }
        .clamp(T_RX_ACK_MIN, T_RX_ACK_MAX);
    }

    fn set_next_frame_number(&mut self) -> u8 {
        let frame_number = self.frame_number;
        self.frame_number = self.next_frame_number();
        frame_number
    }

    fn retransmit(&mut self) -> std::io::Result<()> {
        while self.sent_data.len() < ACK_TIMEOUTS - 1 {
            if let Some(mut data) = self.retransmit.pop_front() {
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
        while self.sent_data.len() < ACK_TIMEOUTS - 1 {
            if let Some(chunk) = chunks.pop() {
                debug!("Transmitting chunk.");
                let data =
                    Data::try_from((self.set_next_frame_number(), chunk.collect_vec().into()))?;
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
        for ack_number in self.pending_acks() {
            self.send_ack(ack_number)?;
        }

        Ok(())
    }

    fn reject(&mut self) -> std::io::Result<()> {
        self.reject = true;
        self.send_nak()
    }

    fn send_ack(&mut self, ack_number: u8) -> std::io::Result<()> {
        self.send_frame(&Ack::from_ack_num(ack_number))?;
        self.last_sent_ack = ack_number;
        Ok(())
    }

    fn send_data(&mut self, data: Data) -> std::io::Result<()> {
        self.send_frame(&data)?;
        trace!("Sending data frame with payload: {:#04X?}", data.payload());
        trace!(
            "Sending data frame with unmasked payload: {:#04X?}",
            data.payload().iter().copied().mask().collect_vec()
        );
        self.sent_data.push((SystemTime::now(), data));
        Ok(())
    }

    fn send_nak(&mut self) -> std::io::Result<()> {
        self.last_received_frame_number.map_or_else(
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
        self.send_buffer.clear();
        self.send_buffer.extend(frame.into_iter().stuff());
        self.send_buffer.push(FLAG);
        trace!("Sending bytes: {:#04X?}", self.send_buffer);
        self.serial_port.write_all(&self.send_buffer)
    }

    fn ack_sent_data(&mut self, ack_num: u8) {
        debug!("Acknowledged frame: {ack_num}");

        if let Some((timestamp, _)) = self
            .sent_data
            .iter()
            .filter(|(_, data)| data.frame_num() < ack_num)
            .sorted_by_key(|(timestamp, _)| timestamp)
            .next_back()
        {
            self.update_t_rx_ack(SystemTime::now().duration_since(*timestamp).ok());
        }

        self.sent_data.retain(|(_, data)| {
            (data.frame_num() >= ack_num) && !((ack_num == 0) && (data.frame_num() == 7))
        });
        trace!("Unacknowledged data after ACK: {:#04X?}", self.sent_data);
    }

    fn receive_packet(&mut self) -> Result<Packet, crate::Error> {
        Ok(Packet::try_from(
            self.receive_frame()?
                .iter()
                .copied()
                .unstuff()
                .collect_vec()
                .as_slice(),
        )?)
    }

    fn receive_frame(&mut self) -> Result<&[u8], crate::Error> {
        self.receive_buffer.clear();
        let mut error = false;

        while !self.terminate.load(Ordering::SeqCst) {
            self.serial_port.read_exact(&mut self.byte_buffer)?;

            match self.byte_buffer[0] {
                CANCEL => {
                    self.receive_buffer.clear();
                    error = false;
                }
                FLAG => {
                    if !error && !self.receive_buffer.is_empty() {
                        debug!("Received frame.");
                        trace!("Frame details: {:#04X?}", self.receive_buffer);
                        return Ok(&self.receive_buffer);
                    }

                    self.receive_buffer.clear();
                    error = false;
                }
                SUBSTITUTE => {
                    error = true;
                }
                X_ON => {
                    info!("NCP requested to stop transmission.");
                    self.transmit = true;
                }
                X_OFF => {
                    info!("NCP requested to resume transmission.");
                    self.transmit = false;
                }
                TIMEOUT => {
                    warn!("Received timeout byte not specified in protocol definition.");
                }
                byte => self.receive_buffer.push(byte),
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
                Packet::RstAck(rst_ack) => {
                    debug!("Received frame: {rst_ack}");
                    trace!("Frame details: {rst_ack:#04X?}");
                    return Ok(());
                }
                Packet::Rst(rst) => {
                    error!("NCP sent us a RST instead of a RSTACK.");
                    debug!("Received frame: {rst}");
                    trace!("Frame details: {rst:#04X?}");
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
                Err(error) => warn!("Startup attempt #{attempt} failed: {error}"),
            }

            sleep(T_REMOTE_NOTRDY);
        }

        error!("Startup failed after {MAX_STARTUP_ATTEMPTS} tries.");
        Err(crate::Error::InitializationFailed)
    }

    fn clear_buffers(&mut self) {
        debug!("Clearing buffers.");
        self.sent_data.clear();
        self.retransmit.clear();
        self.received_data.clear();
        self.receive_buffer.clear();
        self.byte_buffer = [0];
        self.send_buffer.clear();
    }

    fn received_bytes(&self) -> Arc<[u8]> {
        self.received_data
            .iter()
            .dedup_by(|(_, lhs), (_, rhs)| lhs.frame_num() == rhs.frame_num())
            .flat_map(|(_, data)| data.payload().iter().copied().mask())
            .collect()
    }

    fn is_transaction_complete(&self, chunks: &Chunks) -> bool {
        debug!("Checking whether transaction is complete:");
        debug!("Pending ACKs: {:#04X?}", self.pending_acks().is_empty());
        debug!("Sent data empty: {}", self.sent_data.is_empty());
        debug!("Retransmit queue empty: {}", self.retransmit.is_empty());
        chunks.is_empty()
            && self.pending_acks().is_empty()
            && self.sent_data.is_empty()
            && self.retransmit.is_empty()
    }

    const fn pending_acks(&self) -> RangeInclusive<u8> {
        let first = next_three_bit_number(self.last_sent_ack);
        let last = self.ack_number();

        if first == 0 && last == 7 {
            last..=first
        } else {
            first..=last
        }
    }

    const fn ack_number(&self) -> u8 {
        if let Some(last_received_frame_number) = self.last_received_frame_number {
            next_three_bit_number(last_received_frame_number)
        } else {
            0
        }
    }

    const fn next_frame_number(&self) -> u8 {
        next_three_bit_number(self.frame_number)
    }
}

const fn next_three_bit_number(number: u8) -> u8 {
    (number + 1) % 8
}

fn process_rst_ack(rst_ack: &RstAck) {
    debug!("Received frame: {rst_ack}");
    trace!("Frame details: {rst_ack:#04X?}");
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
