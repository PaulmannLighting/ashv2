use crate::status::Status;
use crate::Transceiver;
use log::trace;

impl Transceiver {
    pub(in crate::transceiver) fn enter_reject(&mut self) -> std::io::Result<()> {
        trace!("Entering rejection state.");
        self.state.reject = true;
        self.nak()
    }

    pub(in crate::transceiver) fn leave_reject(&mut self) {
        trace!("Leaving rejection state.");
        self.buffers.clear();
        self.state.reject = false;
        self.state.last_received_frame_num = None;
        self.state.status = Status::Connected;
    }
}
