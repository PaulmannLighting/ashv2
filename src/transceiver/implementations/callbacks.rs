//! Handle callbacks actively sent by the NCP outside of transactions.

use crate::Transceiver;
use serialport::SerialPort;

impl<T> Transceiver<T>
where
    T: SerialPort,
{
    /// Handle callbacks actively sent by the NCP outside of transactions.
    pub(in crate::transceiver) fn handle_callbacks(&mut self) -> std::io::Result<()> {
        while let Some(callback) = self.receive()? {
            self.handle_packet(callback)?;
        }

        Ok(())
    }
}
