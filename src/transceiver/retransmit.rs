use crate::packet::Data;
use std::time::{Duration, SystemTime, SystemTimeError};

const T_RX_ACK_MAX: Duration = Duration::from_millis(3200);

#[derive(Debug)]
pub struct Retransmit {
    sent: SystemTime,
    data: Data,
}

impl Retransmit {
    #[must_use]
    pub fn sent(&self) -> SystemTime {
        self.sent
    }

    pub fn elapsed(&self) -> Result<Duration, SystemTimeError> {
        self.sent.elapsed()
    }

    pub fn is_timed_out(&self) -> bool {
        self.elapsed()
            .map(|elapsed| elapsed > T_RX_ACK_MAX)
            .unwrap_or(true)
    }

    #[must_use]
    pub fn into_data(mut self) -> Data {
        self.data.set_is_retransmission(true);
        self.data
    }
}

impl From<Data> for Retransmit {
    fn from(data: Data) -> Self {
        Self {
            sent: SystemTime::now(),
            data,
        }
    }
}
