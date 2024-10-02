use crate::packet::Packet;
use crate::status::Status;
use crate::Transceiver;
use log::debug;
use std::time::SystemTime;

impl Transceiver {
    pub(in crate::transceiver) fn connect(&mut self) -> std::io::Result<()> {
        debug!("Connecting to NCP...");
        let start = SystemTime::now();

        loop {
            self.rst()?;

            if let Packet::RstAck(rst_ack) = self.read_packet()? {
                debug!("Received RSTACK: {rst_ack}");
                self.state.status = Status::Connected;

                if let Ok(elapsed) = start.elapsed() {
                    debug!("Connection established after {elapsed:?}");
                }

                return Ok(());
            }
        }
    }
}
