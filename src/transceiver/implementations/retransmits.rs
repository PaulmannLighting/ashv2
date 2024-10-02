use crate::packet::Data;
use crate::transceiver::constants::{T_RX_ACK_MAX, T_RX_ACK_MIN};
use crate::wrapping_u3::WrappingU3;
use crate::Transceiver;
use log::{debug, trace};
use serialport::SerialPort;
use std::io::{Error, ErrorKind};
use std::time::{Duration, SystemTime};

impl<T> Transceiver<T>
where
    T: SerialPort,
{
    pub(in crate::transceiver) fn enqueue_retransmit(&mut self, data: Data) -> std::io::Result<()> {
        self.buffers
            .retransmits
            .insert(0, data.into())
            .map_err(|_| {
                Error::new(
                    ErrorKind::OutOfMemory,
                    "ASHv2: failed to enqueue retransmit",
                )
            })
    }

    pub(in crate::transceiver) fn nak_sent_packets(
        &mut self,
        nak_num: WrappingU3,
    ) -> std::io::Result<()> {
        trace!("Handling NAK: {nak_num}");
        while let Some(retransmit) = self
            .buffers
            .retransmits
            .iter()
            .position(|retransmit| retransmit.frame_num() == nak_num)
            .map(|index| self.buffers.retransmits.remove(index))
        {
            self.send_data(retransmit.into_data())?;
        }

        Ok(())
    }

    pub(in crate::transceiver) fn retransmit_timed_out_data(&mut self) -> std::io::Result<()> {
        while let Some(retransmit) = self
            .buffers
            .retransmits
            .iter()
            .position(|retransmit| retransmit.is_timed_out(T_RX_ACK_MAX))
            .map(|index| self.buffers.retransmits.remove(index))
        {
            self.send_data(retransmit.into_data())?;
        }

        self.update_t_rx_ack(None);
        Ok(())
    }

    fn update_t_rx_ack(&mut self, last_ack_duration: Option<Duration>) {
        self.state.t_rx_ack = last_ack_duration
            .map_or_else(
                || self.state.t_rx_ack * 2,
                |duration| self.state.t_rx_ack * 7 / 8 + duration / 2,
            )
            .clamp(T_RX_ACK_MIN, T_RX_ACK_MAX);
    }
}
