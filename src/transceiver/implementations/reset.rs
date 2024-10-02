use crate::status::Status;
use crate::transceiver::Transceiver;

impl Transceiver {
    pub(in crate::transceiver) fn reset(&mut self, status: Status) {
        self.buffers.clear();
        self.state.reset(status);
    }

    pub(in crate::transceiver) fn try_clear_reject_condition(&mut self) -> std::io::Result<()> {
        todo!("Clear reject condition")
    }
}
