//! Handling of sent `DATA` frames.
//!
//! This module handles acknowledgement and retransmission of sent `DATA` frames.
//!
//! `ASH` retransmits `DATA` frames if they
//!
//!   * have been `NAK`ed by the NCP or
//!   * not been acknowledged by the NCP in time.
//!
use crate::packet::Data;
use crate::transceiver::constants::{T_RX_ACK_MAX, T_RX_ACK_MIN};
use crate::wrapping_u3::WrappingU3;
use crate::Transceiver;
use log::{debug, trace};
use serialport::SerialPort;
use std::io::{Error, ErrorKind};
use std::time::Duration;

impl<T> Transceiver<T>
where
    T: SerialPort,
{
    /// Enqueue a `DATA` frame for retransmission.
    pub(in crate::transceiver) fn enqueue_sent_data(&mut self, data: Data) -> std::io::Result<()> {
        self.buffers.sent_data.insert(0, data.into()).map_err(|_| {
            Error::new(
                ErrorKind::OutOfMemory,
                "ASHv2: failed to enqueue retransmit",
            )
        })
    }

    /// Remove `DATA` frames from the queue that have been acknowledged by the NCP.
    pub(in crate::transceiver) fn ack_sent_packets(&mut self, ack_num: WrappingU3) {
        while let Some(sent_data) = self
            .buffers
            .sent_data
            .iter()
            .position(|sent_data| sent_data.frame_num() + 1 == ack_num)
            .map(|index| self.buffers.sent_data.remove(index))
        {
            if let Ok(duration) = sent_data.elapsed() {
                trace!("ACKed packet {} after {duration:?}", sent_data.into_data());
                self.update_t_rx_ack(Some(duration));
            } else {
                trace!("ACKed packet {}", sent_data.into_data());
            }
        }
    }

    /// Retransmit `DATA` frames that have been `NAK`ed by the NCP.
    pub(in crate::transceiver) fn nak_sent_packets(
        &mut self,
        nak_num: WrappingU3,
    ) -> std::io::Result<()> {
        trace!("Handling NAK: {nak_num}");

        if let Some(sent_data) = self
            .buffers
            .sent_data
            .iter()
            .position(|sent_data| sent_data.frame_num() == nak_num)
            .map(|index| self.buffers.sent_data.remove(index))
        {
            debug!("Retransmitting NAK'ed packet #{}", sent_data.frame_num());
            self.send_data(sent_data.into_data())?;
        }

        Ok(())
    }

    /// Retransmit `DATA` frames that have not been acknowledged by the NCP in time.
    pub(in crate::transceiver) fn retransmit_timed_out_data(&mut self) -> std::io::Result<()> {
        while let Some(sent_data) = self
            .buffers
            .sent_data
            .iter()
            .position(|sent_data| sent_data.is_timed_out(self.state.t_rx_ack))
            .map(|index| self.buffers.sent_data.remove(index))
        {
            debug!("Retransmitting timed-out packet #{}", sent_data.frame_num());
            self.send_data(sent_data.into_data())?;
        }

        self.update_t_rx_ack(None);
        Ok(())
    }

    /// Update the `T_RX_ACK` timeout duration.
    fn update_t_rx_ack(&mut self, last_ack_duration: Option<Duration>) {
        self.state.t_rx_ack = last_ack_duration
            .map_or_else(
                || self.state.t_rx_ack * 2,
                |duration| self.state.t_rx_ack * 7 / 8 + duration / 2,
            )
            .clamp(T_RX_ACK_MIN, T_RX_ACK_MAX);
        trace!("Updated T_RX_ACK to {:?}", self.state.t_rx_ack);
    }
}
