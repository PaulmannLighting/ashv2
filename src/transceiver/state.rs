use std::time::Duration;

use log::trace;

use crate::status::Status;
use crate::transceiver::constants::{T_RX_ACK_INIT, T_RX_ACK_MAX, T_RX_ACK_MIN};
use crate::utils::WrappingU3;

/// The state of the transceiver.
#[derive(Debug)]
pub struct State {
    status: Status,
    frame_number: WrappingU3,
    last_received_frame_num: Option<WrappingU3>,
    reject: bool,
    t_rx_ack: Duration,
}

impl State {
    pub const fn new() -> Self {
        Self {
            status: Status::Disconnected,
            frame_number: WrappingU3::ZERO,
            last_received_frame_num: None,
            reject: false,
            t_rx_ack: T_RX_ACK_INIT,
        }
    }

    /// Returns the current status of the `ASHv2` connection.
    pub const fn status(&self) -> Status {
        self.status
    }

    /// Sets the current status of the `ASHv2` connection.
    pub const fn set_status(&mut self, status: Status) {
        self.status = status;
    }

    /// Sets the last received frame number.
    pub const fn set_last_received_frame_num(&mut self, frame_num: WrappingU3) {
        self.last_received_frame_num.replace(frame_num);
    }

    /// Returns whether the transceiver is rejecting frames.
    pub const fn reject(&self) -> bool {
        self.reject
    }

    /// Sets whether the transceiver is rejecting frames.
    pub const fn set_reject(&mut self, reject: bool) {
        self.reject = reject;
    }

    /// Returns the `T_RX_ACK` timeout duration.
    pub const fn t_rx_ack(&self) -> Duration {
        self.t_rx_ack
    }

    /// Update the `T_RX_ACK` timeout duration.
    pub fn update_t_rx_ack(&mut self, last_ack_duration: Option<Duration>) {
        self.t_rx_ack = last_ack_duration
            .map_or_else(
                || self.t_rx_ack * 2,
                |duration| self.t_rx_ack * 7 / 8 + duration / 2,
            )
            .clamp(T_RX_ACK_MIN, T_RX_ACK_MAX);
        trace!("Updated T_RX_ACK to {:?}", self.t_rx_ack);
    }

    /// Returns the next frame number.
    pub fn next_frame_number(&mut self) -> WrappingU3 {
        let frame_number = self.frame_number;
        self.frame_number += 1;
        frame_number
    }

    /// Returns the ACK number.
    ///
    /// This is equal to the last received frame number plus one.
    pub fn ack_number(&self) -> WrappingU3 {
        self.last_received_frame_num
            .map_or_else(WrappingU3::default, |ack_number| ack_number + 1)
    }

    /// Resets the transceiver state.
    pub const fn reset(&mut self, status: Status) {
        self.status = status;
        self.frame_number = WrappingU3::ZERO;
        self.last_received_frame_num = None;
        self.reject = false;
        self.t_rx_ack = T_RX_ACK_INIT;
    }
}
