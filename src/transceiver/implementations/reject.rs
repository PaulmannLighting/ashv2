use crate::Transceiver;
use log::trace;

impl Transceiver {
    pub(in crate::transceiver) fn enter_reject(&mut self) -> std::io::Result<()> {
        if self.state.reject {
            Ok(())
        } else {
            trace!("Entering rejection state.");
            self.state.reject = true;
            self.nak()
        }
    }

    pub(in crate::transceiver) fn leave_reject(&mut self) {
        if self.state.reject {
            trace!("Leaving rejection state.");
            self.state.reject = false;
        }
    }
}
