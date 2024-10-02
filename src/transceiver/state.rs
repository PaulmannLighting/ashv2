use crate::status::Status;
use crate::wrapping_u3::WrappingU3;
use std::time::SystemTime;

/// The state of the transceiver.
#[derive(Debug, Default)]
pub struct State {
    pub(super) status: Status,
    pub(super) last_n_rdy_transmission: Option<SystemTime>,
    pub(super) frame_number: WrappingU3,
    pub(super) last_received_frame_num: Option<WrappingU3>,
    pub(super) reject: bool,
    pub(super) within_transaction: bool,
}

impl State {
    /// Returns the next frame number.
    pub(in crate::transceiver) fn next_frame_number(&mut self) -> WrappingU3 {
        let frame_number = self.frame_number;
        self.frame_number += 1;
        frame_number
    }

    /// Resets the transceiver state.
    pub(in crate::transceiver) fn reset(&mut self, status: Status) {
        self.status = status;
        self.last_n_rdy_transmission = None;
        self.frame_number = WrappingU3::from_u8_lossy(0);
        self.last_received_frame_num = None;
        self.reject = false;
        self.within_transaction = false;
    }
}
