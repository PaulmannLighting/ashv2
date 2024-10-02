use crate::Transceiver;
use serialport::SerialPort;

impl<T> Transceiver<T>
where
    T: SerialPort,
{
    pub(in crate::transceiver) fn handle_callbacks(&mut self) -> std::io::Result<()> {
        self.buffers.response.clear();

        while let Some(callback) = self.receive()? {
            self.handle_packet(&callback)?;
        }

        if !self.buffers.response.is_empty() {
            self.channels
                .callback(self.buffers.response.clone().into())?;
        };

        Ok(())
    }
}
