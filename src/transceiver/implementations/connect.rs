use crate::ash_read::AshRead;
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
            self.reset()?;

            if let Packet::RstAck(rst_ack) = self
                .serial_port
                .read_packet_buffered(&mut self.frame_buffer)?
            {
                debug!("Received RSTACK: {rst_ack}");
                self.status = Status::Connected;

                if let Ok(elapsed) = start.elapsed() {
                    debug!("Connection established after {elapsed:?}");
                }

                return Ok(());
            }
        }
    }
}
