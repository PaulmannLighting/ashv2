use crate::transceiver::Transceiver;
use crate::wrapping_u3::WrappingU3;

impl Transceiver {
    pub(in crate::transceiver) fn ack_number(&self) -> WrappingU3 {
        self.state
            .last_received_frame_num
            .map_or_else(WrappingU3::default, |ack_number| ack_number + 1)
    }

    pub(in crate::transceiver) fn n_rdy(&self) -> bool {
        if !self.state.within_transaction {
            return false;
        }

        if let Some(timestamp) = self.state.last_n_rdy_transmission {
            if let Ok(elapsed) = timestamp.elapsed() {
                return elapsed > Self::T_REMOTE_NOTRDY / 2;
            }
        }

        false
    }
}
