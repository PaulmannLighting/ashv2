//! Packet handling implementation for the transceiver.
//!
//! This module contains methods to handle incoming packets sent by the NCP.
use crate::frame::Frame;
use crate::packet::{Ack, Data, Error, Nak, Packet, RstAck};
use crate::status::Status;
use crate::Transceiver;
use log::{debug, error, trace, warn};
use serialport::SerialPort;
use std::io::ErrorKind;

impl<T> Transceiver<T>
where
    T: SerialPort,
{
    /// Handle an incoming packet.
    ///
    /// # Errors
    ///
    /// Returns an [Error] if the packet handling failed.
    pub(in crate::transceiver) fn handle_packet(&mut self, packet: Packet) -> std::io::Result<()> {
        debug!("Handling: {packet}");
        trace!("{packet:#04X?}");

        if self.state.status == Status::Connected {
            match packet {
                Packet::Ack(ref ack) => self.handle_ack(ack),
                Packet::Data(data) => self.handle_data(data)?,
                Packet::Error(ref error) => return Err(Self::handle_error(error)),
                Packet::Nak(ref nak) => self.handle_nak(nak)?,
                Packet::RstAck(ref rst_ack) => return Err(Self::handle_rst_ack(rst_ack)),
                Packet::Rst(_) => warn!("Received unexpected RST from NCP."),
            }
        } else {
            warn!("Not connected. Dropping frame: {packet}");
        }

        Ok(())
    }

    /// Handle an incoming `ACK` packet.
    fn handle_ack(&mut self, ack: &Ack) {
        if !ack.is_crc_valid() {
            warn!("Received ACK with invalid CRC.");
        }

        self.ack_sent_packets(ack.ack_num());
    }

    /// Handle an incoming `DATA` packet.
    fn handle_data(&mut self, data: Data) -> std::io::Result<()> {
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
            warn!("Received retransmission of frame: {data}");
            self.ack_sent_packets(data.ack_num());
            self.buffers.extend_response(data.into_payload());
        } else {
            debug!("Received out-of-sequence data frame: {data}");
            self.enter_reject()?;
        }

        Ok(())
    }

    /// Handle an incoming `ERROR` packet.
    fn handle_error(error: &Error) -> std::io::Error {
        if !error.is_ash_v2() {
            error!("{error} is not ASHv2: {:#04X}", error.version());
        }

        error.code().map_or_else(
            |code| {
                error!("NCP sent ERROR with invalid code: {code}");
            },
            |code| {
                warn!("NCP sent ERROR condition: {code}");
            },
        );

        std::io::Error::new(
            ErrorKind::ConnectionReset,
            "ASHv2: NCP entered ERROR state.",
        )
    }

    /// Handle an incoming `NAK` packet.
    fn handle_nak(&mut self, nak: &Nak) -> std::io::Result<()> {
        if !nak.is_crc_valid() {
            warn!("Received ACK with invalid CRC.");
        }

        self.nak_sent_packets(nak.ack_num())
    }

    /// Handle an incoming `RSTACK` packet.
    fn handle_rst_ack(rst_ack: &RstAck) -> std::io::Error {
        error!("Received unexpected RSTACK: {rst_ack}");

        if !rst_ack.is_ash_v2() {
            error!("{rst_ack} is not ASHv2: {:#04X}", rst_ack.version());
        }

        rst_ack.code().map_or_else(
            |code| {
                error!("NCP sent RSTACK with unknown code: {code}");
            },
            |code| {
                warn!("NCP sent RSTACK condition: {code}");
            },
        );

        std::io::Error::new(
            ErrorKind::ConnectionReset,
            "ASHv2: NCP received unexpected RSTACK.",
        )
    }
}
