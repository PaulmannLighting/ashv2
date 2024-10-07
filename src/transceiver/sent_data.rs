use crate::packet::Data;
use crate::transceiver::constants::ACK_TIMEOUTS;
use crate::wrapping_u3::WrappingU3;
use std::io::{Error, ErrorKind};
use std::time::{Duration, SystemTime, SystemTimeError};

#[derive(Debug)]
pub struct SentData {
    sent: SystemTime,
    data: Data,
    transmits: usize,
}

impl SentData {
    #[must_use]
    pub const fn frame_num(&self) -> WrappingU3 {
        self.data.frame_num()
    }

    pub fn elapsed(&self) -> Result<Duration, SystemTimeError> {
        self.sent.elapsed()
    }

    #[must_use]
    pub fn is_timed_out(&self, threshold: Duration) -> bool {
        self.elapsed()
            .map(|elapsed| elapsed > threshold)
            .unwrap_or(true)
    }

    pub fn data_for_transmit(&mut self) -> std::io::Result<&Data> {
        self.transmits += 1;

        if self.transmits > 1 {
            self.data.set_is_retransmission(true);
        }

        if self.transmits >= ACK_TIMEOUTS {
            return Err(Error::new(
                ErrorKind::TimedOut,
                format!(
                    "retransmission limit of frame #{} exceeded",
                    self.data.frame_num()
                ),
            ));
        }

        Ok(&self.data)
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
            transmits: 0,
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::packet::Data;
    use crate::transceiver::sent_data::SentData;
    use crate::wrapping_u3::WrappingU3;

    #[test]
    fn test_new() {
        let data = Data::new(
            WrappingU3::default(),
            heapless::Vec::new(),
            WrappingU3::default(),
        );
        let sent_data: SentData = data.into();
        assert_eq!(sent_data.transmits, 0);
        assert!(!sent_data.data.is_retransmission());
    }

    #[test]
    fn test_transmit() {
        let data = Data::new(
            WrappingU3::default(),
            heapless::Vec::new(),
            WrappingU3::default(),
        );
        let mut sent_data: SentData = data.into();
        let data = sent_data.data_for_transmit().unwrap();
        assert!(!data.is_retransmission());
        assert_eq!(sent_data.transmits, 1);
    }

    #[test]
    fn test_retransmit() {
        let data = Data::new(
            WrappingU3::default(),
            heapless::Vec::new(),
            WrappingU3::default(),
        );
        let mut sent_data: SentData = data.into();
        let _transmit = sent_data.data_for_transmit().unwrap();
        let retransmit = sent_data.data_for_transmit().unwrap();
        assert!(retransmit.is_retransmission());
        assert_eq!(sent_data.transmits, 2);
    }
}
