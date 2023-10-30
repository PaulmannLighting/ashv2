use super::T_REMOTE_NOTRDY;
use log::{debug, warn};
use std::io::ErrorKind;
use std::ops::RangeInclusive;
use std::time::Duration;

const T_RX_ACK_INIT: Duration = Duration::from_millis(1600);
const T_RX_ACK_MAX: Duration = Duration::from_millis(3200);
const T_RX_ACK_MIN: Duration = Duration::from_millis(400);
const MAX_TIMEOUTS: u32 = 4;

#[derive(Debug, Eq, PartialEq)]
pub struct State {
    initialized: bool,
    frame_number: u8,
    last_received_frame_number: Option<u8>,
    last_sent_ack: u8,
    reject: bool,
    may_transmit: bool,
    t_rx_ack: Duration,
    timeouts: u32,
}

impl State {
    pub const fn ack_number(&self) -> u8 {
        if let Some(last_received_frame_number) = self.last_received_frame_number {
            next_three_bit_number(last_received_frame_number)
        } else {
            0
        }
    }

    pub fn handle_error(&mut self, error: crate::Error) -> Result<Duration, crate::Error> {
        if let crate::Error::Io(io_error) = &error {
            if io_error.kind() == ErrorKind::TimedOut {
                warn!("Reading packet timed out.");
                debug!("{error}");

                if self.timeouts < MAX_TIMEOUTS {
                    self.timeouts += 1;
                    return Ok(T_REMOTE_NOTRDY * self.timeouts);
                }

                self.timeouts = 0;
            }
        }

        Err(error)
    }

    pub const fn initialized(&self) -> bool {
        self.initialized
    }

    pub const fn is_rejecting(&self) -> bool {
        self.reject
    }

    pub const fn may_transmit(&self) -> bool {
        self.may_transmit
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

    pub fn reset(&mut self) {
        self.initialized = false;
        self.frame_number = 0;
        self.last_received_frame_number = None;
        self.last_sent_ack = 0;
        self.reject = false;
        self.may_transmit = true;
        self.t_rx_ack = T_RX_ACK_INIT;
        self.timeouts = 0;
    }

    pub fn set_initialized(&mut self) {
        self.initialized = true;
    }

    pub fn set_last_received_frame_number(&mut self, frame_number: u8) {
        self.last_received_frame_number = Some(frame_number);
    }

    pub fn set_last_sent_ack(&mut self, ack_num: u8) {
        self.last_sent_ack = ack_num;
    }

    pub fn set_rejecting(&mut self, reject: bool) {
        self.reject = reject;
    }

    pub fn set_may_transmit(&mut self, transmit: bool) {
        self.may_transmit = transmit;
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
            may_transmit: true,
            t_rx_ack: T_RX_ACK_INIT,
            timeouts: 0,
        }
    }
}

const fn next_three_bit_number(number: u8) -> u8 {
    (number + 1) % 8
}
