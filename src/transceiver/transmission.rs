use crate::packet::Data;
use crate::transceiver::constants::ACK_TIMEOUTS;
use crate::wrapping_u3::WrappingU3;
use std::io::{Error, ErrorKind};
use std::time::{Duration, SystemTime, SystemTimeError};

#[derive(Debug)]
pub struct Transmission {
    sent: SystemTime,
    data: Data,
    transmits: usize,
}

impl Transmission {
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

impl From<Data> for Transmission {
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
    use crate::transceiver::transmission::Transmission;
    use crate::wrapping_u3::WrappingU3;

    #[test]
    fn test_new() {
        let data = Data::new(
            WrappingU3::default(),
            heapless::Vec::new(),
            WrappingU3::default(),
        );
        let transmission: Transmission = data.into();
        assert_eq!(transmission.transmits, 0);
        assert!(!transmission.data.is_retransmission());
    }

    #[test]
    fn test_transmit() {
        let data = Data::new(
            WrappingU3::default(),
            heapless::Vec::new(),
            WrappingU3::default(),
        );
        let mut transmission: Transmission = data.into();
        let data = transmission.data_for_transmit().unwrap();
        assert!(!data.is_retransmission());
        assert_eq!(transmission.transmits, 1);
    }

    #[test]
    fn test_retransmit() {
        let data = Data::new(
            WrappingU3::default(),
            heapless::Vec::new(),
            WrappingU3::default(),
        );
        let mut transmission: Transmission = data.into();
        let _transmit = transmission.data_for_transmit().unwrap();
        let retransmit = transmission.data_for_transmit().unwrap();
        assert!(retransmit.is_retransmission());
        assert_eq!(transmission.transmits, 2);
    }
}
