use crate::packet::Data;
use crate::wrapping_u3::WrappingU3;
use std::time::{Duration, SystemTime, SystemTimeError};

#[derive(Debug)]
pub struct SentData {
    sent: SystemTime,
    data: Data,
}

impl SentData {
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
    pub fn into_data(self) -> Data {
        self.data
    }
}

impl From<Data> for SentData {
    fn from(data: Data) -> Self {
        Self {
            sent: SystemTime::now(),
            data,
        }
    }
}
