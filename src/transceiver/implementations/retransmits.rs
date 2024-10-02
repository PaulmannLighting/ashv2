use crate::wrapping_u3::WrappingU3;
use crate::Transceiver;
use log::trace;

impl Transceiver {
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
            .position(|retransmit| retransmit.is_timed_out(Self::T_RX_ACK_MAX))
            .map(|index| self.buffers.retransmits.remove(index))
        {
            self.send_data(retransmit.into_data())?;
        }

        Ok(())
    }
}
