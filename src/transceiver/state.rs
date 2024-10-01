use crate::transceiver::status::Status;
use crate::transceiver::T_REMOTE_NOTRDY;
use crate::wrapping_u3::WrappingU3;
use std::time::SystemTime;

/// Transceiver state.
#[derive(Debug)]
pub struct State {
    pub(crate) status: Status,
    pub(crate) last_n_rdy_transmission: Option<SystemTime>,
    pub(crate) frame_number: WrappingU3,
    pub(crate) last_received_frame_num: Option<WrappingU3>,
    pub(crate) reject: bool,
    pub(crate) within_transaction: bool,
}

impl State {
    /// Creates a new transceiver state.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            status: Status::Disconnected,
            last_n_rdy_transmission: None,
            frame_number: WrappingU3::from_u8_lossy(0),
            last_received_frame_num: None,
            reject: false,
            within_transaction: false,
        }
    }

    /// Returns the next frame number.
    #[must_use]
    pub fn next_frame_number(&mut self) -> WrappingU3 {
        let frame_number = self.frame_number;
        self.frame_number += 1;
        frame_number
    }

    /// Returns the ACK number to send.
    #[must_use]
    pub fn ack_number(&self) -> WrappingU3 {
        self.last_received_frame_num
            .map(|frame_num| frame_num + 1)
            .unwrap_or_default()
    }

    /// Determine the `nRDY` flag.
    ///
    /// The NCP may start retransmitting data after `T_REMOTE_NOTRDY`.
    /// So, when within a transaction, we need to send an intermediary ACK/NAK with `nRDY` set.
    ///
    /// We will do this, when the last `nRDY` transmission was more than half of
    /// `T_REMOTE_NOTRDY` ago.
    #[must_use]
    pub fn n_rdy(&self) -> bool {
        if !self.within_transaction {
            return false;
        }

        if let Some(time) = self.last_n_rdy_transmission {
            if let Ok(elapsed) = time.elapsed() {
                return elapsed > T_REMOTE_NOTRDY / 2;
            }
        }

        // If we don't have a timestamp, we assume that we need to send an intermediary ACK/NAK.
        true
    }
}
