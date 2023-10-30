use super::INITIAL_BUFFER_CAPACITY;
use crate::packet::Data;
use crate::protocol::{CANCEL, FLAG};
use crate::util::Extract;
use itertools::Itertools;
use log::{debug, trace, warn};
use std::collections::VecDeque;
use std::time::{Duration, SystemTime};

const ACK_TIMEOUTS: usize = 4;

#[derive(Debug, Eq, PartialEq)]
pub struct Output {
    data: Vec<(SystemTime, Data)>,
    retransmit: VecDeque<Data>,
    buffer: Vec<u8>,
}

impl Output {
    pub fn clear(&mut self) {
        debug!("Clearing output buffers.");
        self.data.clear();
        self.retransmit.clear();
        self.buffer.clear();
    }

    pub fn ack_sent_data(&mut self, ack_num: u8) {
        self.data.retain(|(_, data)| {
            (data.frame_num() >= ack_num) && !((ack_num == 0) && (data.frame_num() == 7))
        });
        trace!("Unacknowledged data after ACK: {:#04X?}", self.data);
    }

    pub fn buffer_frame(&mut self, bytes: impl Iterator<Item = u8>) -> &[u8] {
        self.buffer.clear();
        self.buffer.push(CANCEL);
        self.buffer.extend(bytes);
        self.buffer.push(FLAG);
        trace!("Buffered bytes to sent: {:#04X?}", self.buffer);
        &self.buffer
    }

    pub fn last_ack_duration(&self, ack_num: u8) -> Option<Duration> {
        self.data
            .iter()
            .filter(|(_, data)| data.frame_num() < ack_num)
            .sorted_by_key(|(timestamp, _)| timestamp)
            .next_back()
            .and_then(|(timestamp, _)| SystemTime::now().duration_since(*timestamp).ok())
    }

    pub fn pop_retransmit(&mut self) -> Option<Data> {
        self.retransmit.pop_front()
    }

    pub fn push_data(&mut self, data: Data) {
        self.data.push((SystemTime::now(), data));
    }

    pub fn queue_not_full(&self) -> bool {
        self.data.len() < ACK_TIMEOUTS
    }

    pub fn queues_are_empty(&self) -> bool {
        trace!("Output data queue empty: {}", self.data.is_empty());
        trace!("Retransmit queue empty: {}", self.retransmit.is_empty());
        self.data.is_empty() && self.retransmit.is_empty()
    }

    pub fn queue_retransmit_nak(&mut self, nak_num: u8) {
        for (_, data) in self
            .data
            .extract(|(_, data)| data.frame_num() >= nak_num)
            .into_iter()
            .sorted_by_key(|(_, data)| data.frame_num())
        {
            debug!("Queueing for retransmit due to NAK: {data}");
            trace!("Frame details: {data:#04X?}");
            self.retransmit.push_back(data);
        }
    }

    pub fn queue_retransmit_timeout(&mut self, t_rx_ack: Duration) -> bool {
        let now = SystemTime::now();
        let mut result = false;

        for (_, data) in self.data.extract(|(timestamp, _)| {
            now.duration_since(*timestamp)
                .map_or(false, |duration| duration > t_rx_ack)
        }) {
            warn!("Frame {data} has not been acked in time. Queueing for retransmit.");
            trace!("Frame details: {data:#04X?}");
            self.retransmit.push_back(data);
            result = true;
        }

        result
    }
}

impl Default for Output {
    fn default() -> Self {
        Self {
            data: Vec::new(),
            retransmit: VecDeque::new(),
            buffer: Vec::with_capacity(INITIAL_BUFFER_CAPACITY),
        }
    }
}
