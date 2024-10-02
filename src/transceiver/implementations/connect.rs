use crate::packet::Packet;
use crate::status::Status;
use crate::Transceiver;
use log::{debug, info, trace, warn};
use serialport::SerialPort;
use std::time::SystemTime;

impl<T> Transceiver<T>
where
    T: SerialPort,
{
    pub(in crate::transceiver) fn connect(&mut self) -> std::io::Result<()> {
        debug!("Connecting to NCP...");
        let start = SystemTime::now();
        let mut attempts: usize = 0;

        loop {
            attempts += 1;
            self.rst()?;

            debug!("Waiting for RST_ACK...");
            let packet = loop {
                if let Some(packet) = self.receive()? {
                    break packet;
                }
            };

            match packet {
                Packet::RstAck(rst_ack) => {
                    self.state.status = Status::Connected;
                    info!(
                        "ASHv2 connection established after {attempts} attempt{}.",
                        if attempts > 1 { "s" } else { "" }
                    );

                    if let Ok(elapsed) = start.elapsed() {
                        debug!("Establishing connection took {elapsed:?}");
                    }

                    match rst_ack.code() {
                        Ok(code) => trace!("Received RST_ACK with code: {code}"),
                        Err(code) => warn!("Received RST_ACK with unknown code: {code}"),
                    }

                    return Ok(());
                }
                other => {
                    warn!("Expected RSTACK but got: {other}");
                    continue;
                }
            }
        }
    }
}
