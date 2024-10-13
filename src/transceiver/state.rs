use crate::status::Status;
use crate::transceiver::constants::{T_RX_ACK_INIT, T_RX_ACK_MAX, T_RX_ACK_MIN};
use crate::utils::WrappingU3;
use log::trace;
use std::time::{Duration, SystemTime};

/// The state of the transceiver.
#[derive(Debug)]
pub struct State {
    status: Status,
    last_n_rdy_transmission: Option<SystemTime>,
    frame_number: WrappingU3,
    last_received_frame_num: Option<WrappingU3>,
    reject: bool,
    within_transaction: bool,
    t_rx_ack: Duration,
}

impl State {
    pub const fn new() -> Self {
        Self {
            status: Status::Disconnected,
            last_n_rdy_transmission: None,
            frame_number: WrappingU3::from_u8_lossy(0),
            last_received_frame_num: None,
            reject: false,
            within_transaction: false,
            t_rx_ack: T_RX_ACK_INIT,
        }
    }

    /// Returns the current status of the ASHv2 connection.
    pub const fn status(&self) -> Status {
        self.status
    }

    /// Sets the current status of the ASHv2 connection.
    pub fn set_status(&mut self, status: Status) {
        self.status = status;
    }

    /// Sets the last time a `nRDY` was sent to the NCP.
    pub fn set_last_n_rdy_transmission(&mut self, time: SystemTime) {
        self.last_n_rdy_transmission = Some(time);
    }

    /// Sets the last received frame number.
    pub fn set_last_received_frame_num(&mut self, frame_num: WrappingU3) {
        self.last_received_frame_num.replace(frame_num);
    }

    /// Returns whether the transceiver is rejecting frames.
    pub const fn reject(&self) -> bool {
        self.reject
    }

    /// Sets whether the transceiver is rejecting frames.
    pub fn set_reject(&mut self, reject: bool) {
        self.reject = reject;
    }

    /// Returns whether the transceiver is within a transaction.
    pub const fn within_transaction(&self) -> bool {
        self.within_transaction
    }

    /// Sets whether the transceiver is within a transaction.
    pub fn set_within_transaction(&mut self, within_transaction: bool) {
        self.within_transaction = within_transaction;
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

    /// Returns whether the transceiver is not ready to receive callbacks.
    pub const fn n_rdy(&self) -> bool {
        self.within_transaction
    }

    /// Resets the transceiver state.
    pub fn reset(&mut self, status: Status) {
        self.status = status;
        self.last_n_rdy_transmission = None;
        self.frame_number = WrappingU3::from_u8_lossy(0);
        self.last_received_frame_num = None;
        self.reject = false;
        self.within_transaction = false;
        self.t_rx_ack = T_RX_ACK_INIT;
    }
}
