use std::time::Duration;

#[cfg(any())]
pub const MAX_STARTUP_ATTEMPTS: usize = 5;
pub const ACK_TIMEOUTS: usize = 4;
#[cfg(any())]
pub const TX_K: usize = 5;
pub const SLIDING_WINDOW_SIZE: usize = 1;
pub const T_RX_ACK_INIT: Duration = Duration::from_millis(1600);
pub const T_RX_ACK_MIN: Duration = Duration::from_millis(400);
pub const T_RX_ACK_MAX: Duration = Duration::from_millis(3200);
#[cfg(any())]
pub const T_TX_ACK_DELAY: Duration = Duration::from_millis(20);
#[cfg(any())]
pub const T_REMOTE_NOTRDY: Duration = Duration::from_millis(1000);
pub const T_RSTACK_MAX: Duration = Duration::from_millis(3200);
