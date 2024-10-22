use std::time::SystemTime;

use crate::status::Status;
use crate::utils::WrappingU3;

/// The state of the transceiver.
#[derive(Debug)]
pub struct SharedState {
    status: Status,
    last_n_rdy_transmission: Option<SystemTime>,
    frame_number: WrappingU3,
    last_received_frame_num: Option<WrappingU3>,
    reject: bool,
}

impl SharedState {
    pub const fn new() -> Self {
        Self {
            status: Status::Disconnected,
            last_n_rdy_transmission: None,
            frame_number: WrappingU3::from_u8_lossy(0),
            last_received_frame_num: None,
            reject: false,
        }
    }

    /// Returns the current status of the `ASHv2` connection.
    pub const fn status(&self) -> Status {
        self.status
    }

    /// Sets the current status of the `ASHv2` connection.
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
        self.reject = false;
    }

    /// Returns whether the transceiver is rejecting frames.
    pub const fn reject(&self) -> bool {
        self.reject
    }

    /// Sets whether the transceiver is rejecting frames.
    pub fn set_reject(&mut self, reject: bool) {
        self.reject = reject;
    }

    /// Returns the current frame number.
    pub const fn frame_number(&self) -> WrappingU3 {
        self.frame_number
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
    pub fn reset(&mut self, status: Status) {
        self.status = status;
        self.last_n_rdy_transmission = None;
        self.frame_number = WrappingU3::from_u8_lossy(0);
        self.last_received_frame_num = None;
        self.reject = false;
    }
}
