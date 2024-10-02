use crate::packet::Data;
use crate::wrapping_u3::WrappingU3;
use std::time::{Duration, SystemTime, SystemTimeError};

#[derive(Debug)]
pub struct Retransmit {
    sent: SystemTime,
    data: Data,
}

impl Retransmit {
    #[must_use]
    pub const fn sent(&self) -> SystemTime {
        self.sent
    }

    #[must_use]
    pub const fn frame_num(&self) -> WrappingU3 {
        self.data.frame_num()
    }

    pub fn elapsed(&self) -> Result<Duration, SystemTimeError> {
        self.sent.elapsed()
    }

    pub fn is_timed_out(&self, threshold: Duration) -> bool {
        self.elapsed()
            .map(|elapsed| elapsed > threshold)
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
