use crate::status::Status;
use crate::transceiver::Transceiver;
use log::{error, warn};

impl Transceiver {
    pub(in crate::transceiver) fn reset(&mut self, status: Status) {
        self.buffers.clear();
        self.state.reset(status);
    }

    pub(in crate::transceiver) fn try_clear_reject_condition(&mut self) -> std::io::Result<()> {
        todo!("Clear reject condition")
    }

    pub(in crate::transceiver) fn handle_reset(&mut self, error: std::io::Error) {
        error!("I/O error: {error}");

        if self.state.within_transaction {
            warn!("Aborting current transaction with error.");
            self.channels.respond(Err(error)).unwrap_or_else(drop);
        }

        self.reset(Status::Failed);
    }
}
