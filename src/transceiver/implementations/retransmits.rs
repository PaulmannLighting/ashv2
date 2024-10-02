use crate::transceiver::constants::{T_RX_ACK_MAX, T_RX_ACK_MIN};
use crate::wrapping_u3::WrappingU3;
use crate::Transceiver;
use log::trace;
use serialport::SerialPort;
use std::time::{Duration, SystemTime};

impl<T> Transceiver<T>
where
    T: SerialPort,
{
    pub(in crate::transceiver) fn ack_sent_packets(&mut self, ack_num: WrappingU3) {
        trace!("Handling ACK: {ack_num}");
        while let Some(retransmit) = self
            .buffers
            .retransmits
            .iter()
            .position(|retransmit| retransmit.frame_num() + 1 == ack_num)
            .map(|index| self.buffers.retransmits.remove(index))
        {
            if let Ok(duration) = SystemTime::now().duration_since(retransmit.sent_at()) {
                self.update_t_rx_ack(Some(duration));
            }

            trace!("ACKed packet #{}", retransmit.into_data().frame_num());
        }
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
