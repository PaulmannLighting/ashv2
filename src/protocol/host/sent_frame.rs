use crate::packet::data::Data;
use std::time::{Duration, SystemTime, SystemTimeError};

#[derive(Debug)]
pub struct SentFrame {
    frame: Data,
    timestamp: SystemTime,
}

impl SentFrame {
    pub fn duration(&self) -> Result<Duration, SystemTimeError> {
        SystemTime::now().duration_since(self.timestamp)
    }
}

impl From<Data> for SentFrame {
    fn from(frame: Data) -> Self {
        Self {
            frame,
            timestamp: SystemTime::now(),
        }
    }
}
