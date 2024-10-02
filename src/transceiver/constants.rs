use std::time::Duration;

#[cfg(any())]
pub(super) const MAX_STARTUP_ATTEMPTS: usize = 5;
#[cfg(any())]
pub(super) const ACK_TIMEOUTS: usize = 4;
pub(super) const TX_K: usize = 5;
pub(super) const T_RX_ACK_INIT: Duration = Duration::from_millis(1600);
pub(super) const T_RX_ACK_MIN: Duration = Duration::from_millis(400);
pub(super) const T_RX_ACK_MAX: Duration = Duration::from_millis(3200);
#[cfg(any())]
pub(super) const T_TX_ACK_DELAY: Duration = Duration::from_millis(20);
#[cfg(any())]
pub(super) const T_REMOTE_NOTRDY: Duration = Duration::from_millis(1000);
pub(super) const T_RSTACK_MAX: Duration = Duration::from_millis(3200);
