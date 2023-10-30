use crate::frame::Frame;
use crate::packet::{Ack, Data, Error, Nak, Packet, Rst, RstAck};
use crate::protocol::host2::ash_receiver::AshReceiver;
use crate::protocol::host2::ash_sender::AshSender;
use crate::protocol::host2::transaction::BytesIO;
use crate::protocol::host2::Transaction;
use crate::protocol::{AshChunks, Mask};
use crate::util::Extract;
use itertools::Itertools;
use log::{debug, error, trace, warn};
use serialport::SerialPort;
use std::collections::VecDeque;
use std::fmt::Debug;
use std::ops::RangeInclusive;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering::SeqCst;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::Arc;
use std::thread::{sleep, spawn, JoinHandle};
use std::time::{Duration, SystemTime};

const MAX_STARTUP_ATTEMPTS: u8 = 5;
const T_RSTACK_MAX: Duration = Duration::from_millis(3200);
const T_REMOTE_NOTRDY: Duration = Duration::from_millis(1000);

const T_RX_ACK_INIT: Duration = Duration::from_millis(1600);
const T_RX_ACK_MAX: Duration = Duration::from_millis(3200);
const T_RX_ACK_MIN: Duration = Duration::from_millis(400);
const ACK_TIMEOUTS: usize = 4;

#[derive(Debug)]
pub struct Worker<S>
where
    S: SerialPort,
{
    // Shared state
    serial_port: S,
    receiver: Receiver<Transaction>,
    incoming: Option<Receiver<Result<Packet, crate::Error>>>,
    outgoing: Option<Sender<Packet>>,
    terminate: Arc<AtomicBool>,
    may_transmit: Arc<AtomicBool>,
    // Local state
    ash_sender: Option<JoinHandle<()>>,
    ash_receiver: Option<JoinHandle<()>>,
    sent_queue: VecDeque<(SystemTime, Data)>,
    retransmit_queue: VecDeque<Data>,
    last_ack_duration: Option<Duration>,
    current_transaction: Option<BytesIO>,
    current_chunks: VecDeque<Vec<u8>>,
    received_data: Vec<Data>,
    is_rejecting: bool,
    is_connected: bool,
    frame_num: u8,
    last_received_frame_number: Option<u8>,
    last_sent_ack: u8,
    t_rx_ack: Duration,
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
            serial_port,
            receiver,
            incoming: None,
            outgoing: None,
            terminate,
            may_transmit: Arc::new(AtomicBool::new(false)),
            ash_sender: None,
            ash_receiver: None,
            sent_queue: VecDeque::new(),
            retransmit_queue: VecDeque::new(),
            current_transaction: None,
            current_chunks: VecDeque::new(),
            received_data: Vec::new(),
            last_ack_duration: None,
            is_rejecting: false,
            is_connected: false,
            frame_num: 0,
            last_received_frame_number: None,
            last_sent_ack: 0,
            t_rx_ack: T_RX_ACK_INIT,
        }
    }

    pub fn spawn(mut self) {
        self.spawn_sender();
        self.spawn_receiver();

        if !self.is_connected && !self.initialize() {
            error!("ASH initialization failed. Bailing out.");
            self.terminate.store(true, SeqCst);
        }

        while !self.terminate.load(SeqCst) {
            self.handle_incoming_packets();
            self.send_pending_acks();
            self.send_retransmits();

            if self.current_transaction.is_none() {
                self.process_next_transaction();
            } else {
                self.push_chunks();
            }

            self.try_complete_current_transaction();
            sleep(Duration::from_millis(500));
        }
    }

    fn spawn_sender(&mut self) {
        let (packet_tx_sender, packet_tx_receiver) = channel();
        let sender = AshSender::new(
            self.serial_port
                .try_clone()
                .expect("Could not clone serial port for receiver."),
            packet_tx_receiver,
            self.terminate.clone(),
        );
        self.ash_sender = Some(spawn(|| sender.spawn()));
        self.outgoing = Some(packet_tx_sender);
    }

    fn spawn_receiver(&mut self) {
        let (packet_rx_sender, packet_rx_receiver) = channel();
        let receiver = AshReceiver::new(
            self.serial_port
                .try_clone()
                .expect("Could not clone serial port for receiver."),
            self.terminate.clone(),
            packet_rx_sender,
            self.may_transmit.clone(),
        );
        self.ash_receiver = Some(spawn(|| receiver.spawn()));
        self.incoming = Some(packet_rx_receiver);
    }

    fn initialize(&mut self) -> bool {
        for attempt in 1..=MAX_STARTUP_ATTEMPTS {
            if self.reset() {
                debug!("ASH connection initialized after {attempt} attempts.");
                return true;
            }

            warn!("Startup attempt #{attempt} failed.");
            sleep(T_REMOTE_NOTRDY);
        }

        error!("Startup failed after {MAX_STARTUP_ATTEMPTS} tries.");
        false
    }

    fn handle_incoming_packets(&mut self) {
        debug!("Handling incoming packets.");

        if let Some(incoming) = self.incoming.take() {
            for result in incoming.try_iter() {
                self.handle_incoming_result(result);
            }

            self.incoming = Some(incoming);
        }
    }

    fn handle_incoming_result(&mut self, result: Result<Packet, crate::Error>) {
        match result {
            Ok(packet) => self.handle_incoming_packet(packet),
            Err(error) => {
                error!("{error}");
                if let Some(transaction) = self.current_transaction.take() {
                    transaction.resolve(Err(error));
                }
            }
        }
    }

    fn handle_incoming_packet(&mut self, packet: Packet) {
        if packet.is_crc_valid() {
            match packet {
                Packet::Ack(ref ack) => self.process_ack(ack),
                Packet::Data(data) => self.process_data(data),
                Packet::Error(ref error) => self.handle_error(error),
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
            self.send_nak();
        }
    }

    fn process_ack(&mut self, ack: &Ack) {
        debug!("Received frame: {ack}");
        trace!("Frame details: {ack:#04X?}");
        self.ack_sent_data(ack.ack_num());
    }

    fn ack_sent_data(&mut self, ack_num: u8) {
        debug!("Acknowledged frame: {ack_num}");

        if let Some(duration) = self.last_ack_duration(ack_num) {
            debug!("Last ACK duration: {} sec", duration.as_secs_f32());
            self.update_t_rx_ack(Some(duration));
            debug!("New T_RX_ACK: {} sec", self.t_rx_ack.as_secs_f32());
        }

        self.sent_queue.retain(|(_, data)| {
            (data.frame_num() >= ack_num) && !((ack_num == 0) && (data.frame_num() == 7))
        });
        trace!("Unacknowledged data after ACK: {:#04X?}", self.sent_queue);
    }

    fn last_ack_duration(&self, ack_num: u8) -> Option<Duration> {
        self.sent_queue
            .iter()
            .filter(|(_, data)| data.frame_num() < ack_num)
            .sorted_by_key(|(timestamp, _)| timestamp)
            .next_back()
            .and_then(|(timestamp, _)| SystemTime::now().duration_since(*timestamp).ok())
    }

    // See: 5.6 DATA frame Acknowledgement timing
    pub fn update_t_rx_ack(&mut self, last_ack_duration: Option<Duration>) {
        self.t_rx_ack = if let Some(duration) = last_ack_duration {
            self.t_rx_ack * 7 / 8 + duration / 2
        } else {
            self.t_rx_ack * 2
        }
        .clamp(T_RX_ACK_MIN, T_RX_ACK_MAX);
    }

    fn process_data(&mut self, data: Data) {
        debug!("Received frame: {data}");
        trace!("Frame details: {data:#04X?}");
        trace!(
            "Unmasked payload: {:#04X?}",
            data.payload().iter().copied().mask().collect_vec()
        );

        if data.frame_num() == self.ack_number() {
            self.is_rejecting = false;
            self.last_received_frame_number = Some(data.frame_num());
            debug!("Last received frame number: {}", data.frame_num());
            self.ack_sent_data(data.ack_num());
            self.received_data.push(data);
        } else if data.is_retransmission() {
            self.ack_sent_data(data.ack_num());
            self.received_data.push(data);
        } else {
            debug!("Received out-of-sequence data frame: {data}");

            if !self.is_rejecting {
                self.is_rejecting = true;
                self.send_nak();
            }
        }
    }

    fn handle_error(&mut self, error: &Error) {
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

        self.reset();
    }

    fn reset(&mut self) -> bool {
        // TODO: Since the ports are currently cloned, this will have no effect.
        self.serial_port
            .set_timeout(T_RSTACK_MAX)
            .unwrap_or_else(|error| {
                error!("Could not set timeout on serial port.");
                debug!("{error}");
            });
        self.is_connected = false;
        self.send_packet(Packet::Rst(Rst::default()));

        if let Some(incoming) = &self.incoming {
            for message in incoming.try_iter() {
                match message {
                    Ok(packet) => match packet {
                        Packet::RstAck(ref rst_ack) => {
                            self.reset_state();
                            self.is_connected = true;
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
                            return true;
                        }
                        packet => trace!("Ignoring packet: {packet}."),
                    },
                    Err(error) => {
                        error!("{error}");
                    }
                }
            }
        }

        false
    }

    fn reset_state(&mut self) {
        self.sent_queue.clear();
        self.retransmit_queue.clear();

        if let Some(transaction) = self.current_transaction.take() {
            // TODO: maybe introduce Error::Reset
            transaction.resolve(Err(crate::Error::Terminated));
        }

        self.current_chunks.clear();
        self.received_data.clear();
        self.last_ack_duration = None;
        self.frame_num = 0;
        self.last_received_frame_number = None;
        self.last_sent_ack = 0;
        self.t_rx_ack = T_RX_ACK_INIT;
    }

    fn process_nak(&mut self, nak: &Nak) {
        debug!("Received frame: {nak}");
        trace!("Frame details: {nak:#04X?}");
        self.queue_retransmit_nak(nak.ack_num());
    }

    pub fn queue_retransmit_nak(&mut self, nak_num: u8) {
        for (_, data) in self
            .sent_queue
            .extract(|(_, data)| data.frame_num() >= nak_num)
            .into_iter()
            .sorted_by_key(|(_, data)| data.frame_num())
        {
            debug!("Queueing for retransmit due to NAK: {data}");
            trace!("Frame details: {data:#04X?}");
            self.retransmit_queue.push_back(data);
        }
    }

    fn process_rst_ack(&mut self, rst_ack: &RstAck) {
        debug!("Received frame: {rst_ack}");
        trace!("Frame details: {rst_ack:#04X?}");
        self.may_transmit.store(true, SeqCst);
        self.is_connected = true;
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

    fn send_pending_acks(&mut self) {
        debug!("Sending pending ACKs.");

        for ack_number in self.pending_acks() {
            trace!("Sending ACK #{ack_number}");
            self.send_packet(Packet::Ack(Ack::from_ack_num(ack_number)));
            self.last_sent_ack = ack_number;
        }
    }

    fn send_nak(&mut self) {
        self.last_received_frame_number.map_or_else(
            || {
                error!("No frame received yet. Nothing to reject.");
            },
            |frame_num| {
                self.send_packet(Packet::Nak(Nak::from_ack_num(frame_num)));
            },
        );
    }

    fn send_retransmits(&mut self) {
        debug!("Retransmitting unacknowledged frames.");
        self.queue_retransmit_due_to_timeout();

        if !self.is_connected {
            debug!("Cannot retransmit due to not being connected.");
            return;
        }

        if !self.may_transmit.load(SeqCst) {
            debug!("NCP asked us to not transmit at the moment.");
            return;
        }

        while self.sent_queue.len() < ACK_TIMEOUTS {
            debug!("Buffer space available. Attempting to retransmit frames.");
            if let Some(mut packet) = self.retransmit_queue.pop_front() {
                packet.set_is_retransmission(true);
                debug!("Retransmitting packet: {packet}");
                trace!("Frame details:: {packet:#04X?}");
                self.send_data(packet);
            } else {
                debug!("No more packets to retransmit.");
                break;
            }
        }
    }

    pub fn queue_retransmit_due_to_timeout(&mut self) {
        if let Some(last_ack_duration) = self.last_ack_duration {
            let now = SystemTime::now();

            for (_, data) in self.sent_queue.extract(|(timestamp, _)| {
                now.duration_since(*timestamp)
                    .map_or(false, |duration| duration > last_ack_duration)
            }) {
                warn!("Frame {data} has not been acked in time. Queueing for retransmit.");
                trace!("Frame details: {data:#04X?}");
                self.retransmit_queue.push_back(data);
            }
        }
    }

    fn push_chunks(&mut self) {
        if !self.is_connected {
            debug!("Cannot push chunks due to not being connected.");
            return;
        }

        if !self.may_transmit.load(SeqCst) {
            debug!("NCP asked us to not transmit at the moment.");
            return;
        }

        debug!("Pushing chunks.");
        while self.sent_queue.len() < ACK_TIMEOUTS {
            debug!("Buffer space free. Attempting to send next chunk.");
            if let Some(chunk) = self.current_chunks.pop_back() {
                debug!("Sending chunk.");
                trace!("Chunk: {:#04X?}", chunk);
                self.send_chunk(chunk);
            } else {
                debug!("No more chunks to transmit.");
                break;
            }
        }
    }

    fn send_chunk(&mut self, chunk: Vec<u8>) {
        match Data::try_from((self.get_frame_num_and_increment(), Arc::from(chunk))) {
            Ok(data) => self.send_data(data),
            Err(error) => {
                if let Some(current_transaction) = self.current_transaction.take() {
                    current_transaction.resolve(Err(error.into()));
                }
            }
        }
    }

    fn process_next_transaction(&mut self) {
        debug!("Waiting for next transaction.");
        match self.receiver.recv() {
            Ok(transaction) => self.process_transaction(transaction),
            Err(error) => {
                error!("Failed to receive packet request.");
                debug!("{error}");
            }
        }
    }

    fn process_transaction(&mut self, transaction: Transaction) {
        debug!("Processing transaction.");
        trace!("Transaction: {:#04X?}", transaction);
        match transaction {
            Transaction::Data(data) => self.process_data_request(data),
            Transaction::Reset(reset) => {
                if self.reset() {
                    reset.resolve(Ok(()));
                }
            }
            Transaction::Terminate => self.terminate.store(true, SeqCst),
        }
    }

    fn process_data_request(&mut self, data: BytesIO) {
        self.current_chunks.clear();
        match data.request().iter().copied().ash_chunks() {
            Ok(chunks) => {
                chunks
                    .into_iter()
                    .map(Iterator::collect)
                    .for_each(|chunk| self.current_chunks.push_back(chunk));
                self.current_transaction = Some(data);
            }
            Err(error) => data.resolve(Err(error)),
        }
    }

    fn send_data(&mut self, data: Data) {
        self.send_packet(Packet::Data(data.clone()));
        self.sent_queue.push_back((SystemTime::now(), data));
    }

    fn send_packet(&mut self, packet: Packet) {
        self.outgoing.as_ref().map_or_else(
            || {
                todo!("Handle non-initialized sender");
            },
            |sender| {
                sender
                    .send(packet)
                    .expect("Could not send packet transaction to sender.");
            },
        );
    }

    fn get_frame_num_and_increment(&mut self) -> u8 {
        let frame_num = self.frame_num;
        self.frame_num = next_three_bit_number(frame_num);
        frame_num
    }

    fn ack_number(&self) -> u8 {
        self.last_received_frame_number
            .map_or(0, next_three_bit_number)
    }

    fn pending_acks(&self) -> RangeInclusive<u8> {
        let first = next_three_bit_number(self.last_sent_ack);
        let last = self.ack_number();

        if first == 0 && last == 7 {
            last..=first
        } else {
            first..=last
        }
    }

    fn try_complete_current_transaction(&mut self) {
        if self.transaction_sending_complete() {
            if let Some(transaction) = self.current_transaction.take() {
                if let Some(incoming) = self.incoming.take() {
                    if let Ok(result) = incoming.try_recv() {
                        self.handle_incoming_result(result);
                    } else {
                        transaction.resolve(Ok(self.received_bytes()));
                    }

                    self.incoming = Some(incoming);
                }
            }
        }
    }

    fn transaction_sending_complete(&self) -> bool {
        if !self.current_chunks.is_empty() {
            return false;
        }

        if !self.retransmit_queue.is_empty() {
            return false;
        }

        if !self.sent_queue.is_empty() {
            return false;
        }

        true
    }

    fn received_bytes(&self) -> Arc<[u8]> {
        self.received_data
            .iter()
            .dedup_by(|lhs, rhs| lhs.frame_num() == rhs.frame_num())
            .flat_map(|data| data.payload().iter().copied().mask())
            .collect()
    }
}

impl<S> Drop for Worker<S>
where
    S: SerialPort,
{
    fn drop(&mut self) {
        self.terminate.store(true, SeqCst);

        if let Some(ash_sender) = self.ash_sender.take() {
            ash_sender.join().expect("Could not join sender.");
        }

        if let Some(ash_receiver) = self.ash_receiver.take() {
            ash_receiver.join().expect("Could not join receiver.");
        }
    }
}

const fn next_three_bit_number(number: u8) -> u8 {
    (number + 1) % 8
}
