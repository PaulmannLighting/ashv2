use super::Transceiver;
use log::trace;

impl Transceiver {
    pub(super) fn reject(&mut self) -> std::io::Result<()> {
        trace!("Entering rejection state.");
        self.reject = true;
        self.nak()
    }
}
