use crate::status::Status;
use crate::transceiver::Transceiver;
use log::{error, warn};
use serialport::SerialPort;

impl<T> Transceiver<T>
where
    T: SerialPort,
{
    /// Reset buffers and state.
    pub(in crate::transceiver) fn reset(&mut self) {
        self.buffers.clear();
        self.state.reset(Status::Failed);
    }

    /// Handle I/O errors.
    pub(in crate::transceiver) fn handle_io_error(&mut self, error: std::io::Error) {
        error!("I/O error: {error}");

        if self.state.within_transaction {
            warn!("Aborting current transaction with error.");
            self.channels.respond(Err(error)).unwrap_or_else(drop);
        }

        self.reset();
    }
}
