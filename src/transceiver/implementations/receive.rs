use crate::frame::Frame;
use crate::packet::{Ack, Data, Error, Nak, Packet, RstAck};
use crate::protocol::Mask;
use crate::status::Status;
use crate::Transceiver;
use log::{debug, error, trace, warn};
use serialport::SerialPort;
use std::io::ErrorKind;
use std::time::{Duration, SystemTime};

impl<T> Transceiver<T>
where
    T: SerialPort,
{
    pub(in crate::transceiver) fn receive_with_timeout(
        &mut self,
        timeout: Duration,
    ) -> std::io::Result<Option<Packet>> {
        let start = SystemTime::now();
        loop {
            if let Some(packet) = self.receive()? {
                return Ok(Some(packet));
            }

            if let Ok(elapsed) = start.elapsed() {
                if elapsed >= timeout {
                    return Ok(None);
                }
            } else {
                warn!("System time jumped.");
                return Ok(None);
            }
        }
    }

    pub(in crate::transceiver) fn handle_packet(&mut self, packet: Packet) -> std::io::Result<()> {
        debug!("Received: {packet}");
        trace!("{packet:#04X?}");

        if self.state.status == Status::Connected {
            match packet {
                Packet::Ack(ref ack) => self.handle_ack(ack),
                Packet::Data(data) => self.handle_data(data)?,
                Packet::Error(ref error) => {
                    self.handle_error(error);
                    return Err(std::io::Error::new(
                        ErrorKind::ConnectionReset,
                        "NCP entered ERROR state.",
                    ));
                }
                Packet::Nak(ref nak) => self.handle_nak(nak)?,
                Packet::RstAck(ref rst_ack) => self.handle_rst_ack(rst_ack)?,
                Packet::Rst(_) => warn!("Received unexpected RST from NCP."),
            }
        } else if let Packet::RstAck(ref rst_ack) = packet {
            self.handle_rst_ack(rst_ack)?;
        } else {
            warn!("Not connected. Dropping frame: {packet}");
        }

        Ok(())
    }

    fn handle_ack(&mut self, ack: &Ack) {
        if !ack.is_crc_valid() {
            warn!("Received ACK with invalid CRC.");
        }

        self.ack_sent_packets(ack.ack_num());
    }

    fn handle_data(&mut self, data: Data) -> std::io::Result<()> {
        debug!("Received frame: {data:#04X?}");
        trace!("Unmasked payload: {:#04X?}", {
            let mut unmasked = data.payload().to_vec();
            unmasked.mask();
            unmasked
        });

        if !data.is_crc_valid() {
            warn!("Received data frame with invalid CRC.");
            self.enter_reject()?;
        } else if data.frame_num() == self.state.ack_number() {
            self.leave_reject();
            self.state.last_received_frame_num.replace(data.frame_num());
            self.ack()?;
            self.ack_sent_packets(data.ack_num());
            self.buffers.extend_response(data.into_payload());
        } else if data.is_retransmission() {
            self.ack_sent_packets(data.ack_num());
            self.buffers.extend_response(data.into_payload());
        } else {
            debug!("Received out-of-sequence data frame: {data}");
            self.enter_reject()?;
        }

        Ok(())
    }

    fn handle_error(&mut self, error: &Error) {
        trace!("Received ERROR: {error:#04X?}");

        if !error.is_ash_v2() {
            error!("{error} is not ASHv2: {}", error.version());
        }

        self.state.status = Status::Failed;
        error.code().map_or_else(
            |code| {
                error!("NCP sent error with invalid code: {code}");
            },
            |code| {
                warn!("NCP sent error condition: {code}");
            },
        );
    }

    fn handle_nak(&mut self, nak: &Nak) -> std::io::Result<()> {
        warn!("Received NAK: {nak:#04X?}");

        if !nak.is_crc_valid() {
            warn!("Received ACK with invalid CRC.");
        }

        self.nak_sent_packets(nak.ack_num())
    }

    fn handle_rst_ack(&mut self, rst_ack: &RstAck) -> std::io::Result<()> {
        if !rst_ack.is_ash_v2() {
            error!("{rst_ack} is not ASHv2: {}", rst_ack.version());
        }

        rst_ack.code().map_or_else(
            |code| {
                warn!("NCP acknowledged reset with invalid error code: {code}");
            },
            |code| {
                debug!("NCP acknowledged reset due to: {code}");
            },
        );

        self.leave_reject();
        self.abort_current_command()
    }

    fn abort_current_command(&mut self) -> std::io::Result<()> {
        self.channels.respond(Err(std::io::Error::new(
            ErrorKind::ConnectionReset,
            "NCP reset",
        )))
    }
}
