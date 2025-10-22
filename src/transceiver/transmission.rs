//! Transmitted frame with metadata.

use core::fmt::Display;
use core::time::Duration;
use std::io::{self, Error, ErrorKind};
use std::time::Instant;

use crate::frame::Data;
use crate::transceiver::constants::ACK_TIMEOUTS;
use crate::utils::WrappingU3;

/// A transmitted frame with metadata.
#[derive(Debug)]
pub struct Transmission {
    sent: Instant,
    data: Data,
    transmits: usize,
}

impl Transmission {
    #[must_use]
    pub const fn frame_num(&self) -> WrappingU3 {
        self.data.frame_num()
    }

    pub fn elapsed(&self) -> Duration {
        self.sent.elapsed()
    }

    #[must_use]
    pub fn is_timed_out(&self, threshold: Duration) -> bool {
        self.elapsed() > threshold
    }

    pub fn data_for_transmit(&mut self) -> io::Result<&Data> {
        self.transmits += 1;

        if self.transmits > 1 {
            self.data.set_is_retransmission(true);
        }

        if self.transmits >= ACK_TIMEOUTS {
            return Err(Error::new(
                ErrorKind::TimedOut,
                format!(
                    "Retransmission limit of frame #{} exceeded.",
                    self.data.frame_num()
                ),
            ));
        }

        Ok(&self.data)
    }
}

impl Display for Transmission {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.data.fmt(f)
    }
}

impl From<Data> for Transmission {
    fn from(data: Data) -> Self {
        Self {
            sent: Instant::now(),
            data,
            transmits: 0,
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::frame::Data;
    use crate::transceiver::transmission::Transmission;
    use crate::utils::WrappingU3;

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
