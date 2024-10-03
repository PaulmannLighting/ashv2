//! Establish an `ASHv2` connection with the NCP.

use crate::packet::Packet;
use crate::status::Status;
use crate::transceiver::constants::T_RSTACK_MAX;
use crate::Transceiver;
use log::{debug, error, info, trace, warn};
use serialport::SerialPort;
use std::time::SystemTime;

impl<T> Transceiver<T>
where
    T: SerialPort,
{
    /// Establish an `ASHv2` connection with the NCP.
    pub(in crate::transceiver) fn connect(&mut self) -> std::io::Result<()> {
        debug!("Connecting to NCP...");
        let start = SystemTime::now();
        let mut attempts: usize = 0;

        'attempts: loop {
            attempts += 1;
            self.rst()?;

            debug!("Waiting for RSTACK...");
            let packet = loop {
                if let Some(packet) = self.receive()? {
                    break packet;
                } else if let Ok(elapsed) = start.elapsed() {
                    // Retry sending `RST` if no `RSTACK` was received in time.
                    if elapsed > T_RSTACK_MAX {
                        continue 'attempts;
                    }
                } else {
                    // If the system time jumps, retry sending `RST`.
                    error!("System time jumped.");
                    continue 'attempts;
                }
            };

            match packet {
                Packet::RstAck(rst_ack) => {
                    if !rst_ack.is_ash_v2() {
                        return Err(std::io::Error::new(
                            std::io::ErrorKind::Unsupported,
                            "Received RSTACK is not ASHv2.",
                        ));
                    }

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
