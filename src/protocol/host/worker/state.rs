use std::ops::RangeInclusive;
use std::time::Duration;

const T_RX_ACK_INIT: Duration = Duration::from_millis(1600);
const T_RX_ACK_MAX: Duration = Duration::from_millis(3200);
const T_RX_ACK_MIN: Duration = Duration::from_millis(400);

#[derive(Debug, Eq, PartialEq)]
pub struct State {
    pub initialized: bool,
    frame_number: u8,
    last_received_frame_number: Option<u8>,
    pub last_sent_ack: u8,
    pub reject: bool,
    pub transmit: bool,
    t_rx_ack: Duration,
}

impl State {
    pub const fn ack_number(&self) -> u8 {
        if let Some(last_received_frame_number) = self.last_received_frame_number {
            next_three_bit_number(last_received_frame_number)
        } else {
            0
        }
    }

    pub const fn last_received_frame_number(&self) -> Option<u8> {
        self.last_received_frame_number
    }

    pub fn next_frame_number(&mut self) -> u8 {
        let frame_number = self.frame_number;
        self.frame_number = next_three_bit_number(self.frame_number);
        frame_number
    }

    pub const fn pending_acks(&self) -> RangeInclusive<u8> {
        let first = next_three_bit_number(self.last_sent_ack);
        let last = self.ack_number();

        if first == 0 && last == 7 {
            last..=first
        } else {
            first..=last
        }
    }

    pub fn set_last_received_frame_number(&mut self, frame_number: u8) {
        self.last_received_frame_number = Some(frame_number);
    }

    pub const fn t_rx_ack(&self) -> Duration {
        self.t_rx_ack
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
}

impl Default for State {
    fn default() -> Self {
        Self {
            initialized: false,
            frame_number: 0,
            last_received_frame_number: None,
            last_sent_ack: 0,
            reject: false,
            transmit: true,
            t_rx_ack: T_RX_ACK_INIT,
        }
    }
}

const fn next_three_bit_number(number: u8) -> u8 {
    (number + 1) % 8
}
