use crate::transceiver::Transceiver;
use crate::wrapping_u3::WrappingU3;

impl Transceiver {
    pub(super) fn ack_number(&self) -> WrappingU3 {
        self.last_received_frame_num
            .map_or(WrappingU3::default(), |ack_number| ack_number + 1)
    }

    pub(super) fn n_rdy(&self) -> bool {
        if !self.within_transaction {
            return false;
        }

        if let Some(timestamp) = self.last_n_rdy_transmission {
            if let Ok(elapsed) = timestamp.elapsed() {
                return elapsed > Self::T_REMOTE_NOTRDY / 2;
            }
        }

        false
    }

    pub(super) fn next_frame_number(&mut self) -> WrappingU3 {
        let frame_number = self.frame_number;
        self.frame_number += 1;
        frame_number
    }
}
