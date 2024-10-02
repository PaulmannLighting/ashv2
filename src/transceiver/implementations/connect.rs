use crate::packet::Packet;
use crate::status::Status;
use crate::Transceiver;
use log::{debug, info};
use serialport::SerialPort;
use std::time::SystemTime;

impl<T> Transceiver<T>
where
    T: SerialPort,
{
    pub(in crate::transceiver) fn connect(&mut self) -> std::io::Result<()> {
        debug!("Connecting to NCP...");
        let start = SystemTime::now();

        loop {
            self.rst()?;

            let Some(packet) = self.receive()? else {
                continue;
            };

            match packet {
                Packet::RstAck(rst_ack) => {
                    debug!("Received RSTACK: {rst_ack}");
                    self.state.status = Status::Connected;
                    info!("ASHv2 connection established.");

                    if let Ok(elapsed) = start.elapsed() {
                        debug!("Establishing connection took {elapsed:?}");
                    }

                    return Ok(());
                }
                other => {
                    debug!("Expected RSTACK but got: {other}");
                    continue;
                }
            }
        }
    }
}
