use crate::Transceiver;
use std::time::Duration;

impl Transceiver {
    pub(in crate::transceiver) const MAX_STARTUP_ATTEMPTS: u8 = 5;
    pub(in crate::transceiver) const ACK_TIMEOUTS: usize = 4;
    pub(in crate::transceiver) const TX_K: u8 = 5;
    pub(in crate::transceiver) const T_RX_ACK_INIT: Duration = Duration::from_millis(1600);
    pub(in crate::transceiver) const T_RX_ACK_MIN: Duration = Duration::from_millis(400);
    pub(in crate::transceiver) const T_RX_ACK_MAX: Duration = Duration::from_millis(3200);
    pub(in crate::transceiver) const T_TX_ACK_DELAY: Duration = Duration::from_millis(20);
    pub(in crate::transceiver) const T_REMOTE_NOTRDY: Duration = Duration::from_millis(1000);
    pub(in crate::transceiver) const T_RSTACK_MAX: Duration = Duration::from_millis(3200);
}
