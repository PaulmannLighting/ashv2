use crate::frame::Frame;
use crate::packet::{Ack, Data, Error, Nak, Packet, RstAck};
use crate::protocol::Mask;
use crate::status::Status;
use crate::Transceiver;
use log::{debug, error, trace, warn};
use std::io::ErrorKind;

impl Transceiver {
    /// Receives a packet from the serial port.
    ///
    /// Returns `Ok(None)` if no packet was received within the timeout.
    ///
    /// # Errors
    ///
    /// Returns an error if the serial port read operation failed.
    pub(in crate::transceiver) fn receive(&mut self) -> std::io::Result<Option<Packet>> {
        match self.read_packet() {
            Ok(packet) => Ok(Some(packet)),
            Err(error) => {
                if error.kind() == ErrorKind::TimedOut {
                    Ok(None)
                } else {
                    Err(error)
                }
            }
        }
    }

    pub(in crate::transceiver) fn handle_packet(&mut self, packet: &Packet) -> std::io::Result<()> {
        debug!("Received: {packet}");
        trace!("{packet:#04X?}");

        if self.state.status == Status::Connected {
            match packet {
                Packet::Ack(ref ack) => self.handle_ack(ack),
                Packet::Data(ref data) => self.handle_data(data)?,
                Packet::Error(ref error) => self.handle_error(error),
                Packet::Nak(ref nak) => self.handle_nak(nak)?,
                Packet::RstAck(ref rst_ack) => self.handle_rst_ack(rst_ack)?,
                Packet::Rst(_) => warn!("Received unexpected RST from NCP."),
            }
        } else if let Packet::RstAck(ref rst_ack) = packet {
            self.handle_rst_ack(rst_ack);
        } else {
            warn!("Not connected. Dropping frame: {packet}");
        }

        Ok(())
    }

    fn handle_ack(&mut self, ack: &Ack) {
        if !ack.is_crc_valid() {
            warn!("Received ACK with invalid CRC.");
        }

        self.buffers.ack_sent_packets(ack.ack_num());
    }

    fn handle_data(&mut self, data: &Data) -> std::io::Result<()> {
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
            self.ack(data.frame_num())?;
            self.state.reject = false;
            self.state.last_received_frame_num.replace(data.frame_num());
            debug!("Sending ACK to transmitter: {}", data.ack_num());
            self.buffers.ack_sent_packets(data.ack_num());
            self.buffers.response.extend_from_slice(data.payload());
        } else if data.is_retransmission() {
            debug!("Sending ACK to transmitter: {}", data.ack_num());
            self.buffers.ack_sent_packets(data.ack_num());
            self.buffers.response.extend_from_slice(data.payload());
        } else {
            debug!("Received out-of-sequence data frame: {data}");

            if !self.state.reject {
                self.enter_reject()?;
            }
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
        if !nak.is_crc_valid() {
            warn!("Received ACK with invalid CRC.");
        }

        debug!("Forwarding NAK to transmitter.");
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
