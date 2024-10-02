use crate::packet::Packet;
use crate::status::Status;
use crate::Transceiver;
use log::{debug, info};
use std::time::SystemTime;

impl Transceiver {
    pub(in crate::transceiver) fn connect(&mut self) -> std::io::Result<()> {
        debug!("Connecting to NCP...");
        let start = SystemTime::now();
        let mut attempts: usize = 0;

        loop {
            attempts += 1;
            self.rst()?;

            if let Packet::RstAck(rst_ack) = self.read_packet()? {
                debug!("Received RSTACK: {rst_ack}");
                self.state.status = Status::Connected;
                info!("Connection established after {attempts} attempts.");

                if let Ok(elapsed) = start.elapsed() {
                    debug!("Establishing connection took {elapsed:?}");
                }

                return Ok(());
            }
        }
    }
}
