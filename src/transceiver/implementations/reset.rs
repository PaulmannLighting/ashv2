//! Reset and error handling implementation.
use crate::status::Status;
use crate::transceiver::Transceiver;
use log::error;
use serialport::SerialPort;

impl<T> Transceiver<T>
where
    T: SerialPort,
{
    /// Reset buffers and state.
    pub(in crate::transceiver) fn reset(&mut self) {
        self.channels.reset();
        self.buffers.clear();
        self.state.reset(Status::Failed);
    }

    /// Handle I/O errors.
    pub(in crate::transceiver) fn handle_io_error(&mut self, error: &std::io::Error) {
        error!("I/O error: {error}");

        if self.state.within_transaction {
            error!("Aborting current transaction due to error.");
            self.channels.close();
        }

        self.reset();
    }
}
