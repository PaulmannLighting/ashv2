use crate::transceiver::Transceiver;

impl Transceiver {
    pub(super) fn reset(&mut self) -> std::io::Result<()> {
        todo!("Reset connection")
    }

    pub(super) fn try_clear_reject_condition(&mut self) -> std::io::Result<()> {
        todo!("Clear reject condition")
    }
}
