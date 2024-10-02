use crate::transceiver::Transceiver;

impl Transceiver {
    pub(in crate::transceiver) fn reset(&mut self) -> std::io::Result<()> {
        todo!("Reset connection")
    }

    pub(in crate::transceiver) fn try_clear_reject_condition(&mut self) -> std::io::Result<()> {
        todo!("Clear reject condition")
    }
}
