use crate::Transceiver;
use log::trace;

impl Transceiver {
    pub(in crate::transceiver) fn reject(&mut self) -> std::io::Result<()> {
        trace!("Entering rejection state.");
        self.state.reject = true;
        self.nak()
    }
}
