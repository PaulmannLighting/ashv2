//! Transmitted frame with metadata.

use core::fmt::Display;
use std::io::{self, Error, ErrorKind};
use std::time::Instant;

use crate::frame::Data;

/// One initial send and two retries. The next attempt fails the session.
const MAX_TRANSMISSIONS: usize = 3;

/// A transmitted frame with metadata.
#[derive(Debug)]
pub struct Transmission {
    sent: Instant,
    data: Data,
    transmits: usize,
}

impl Transmission {
    /// Return the frame number of the transmitted data.
    #[must_use]
    pub const fn frame_num(&self) -> u8 {
        self.data.frame_num()
    }

    /// Return when the frame was last written.
    #[must_use]
    pub const fn sent_at(&self) -> Instant {
        self.sent
    }

    /// Record a completed write for the acknowledgement timer.
    pub fn mark_sent(&mut self) {
        self.sent = Instant::now();
    }

    /// Counts this send attempt and refreshes the header and CRC on retransmission.
    ///
    /// # Errors
    ///
    /// Returns an [`Error`] if the retransmission limit is exceeded.
    pub fn data_for_transmit(&mut self, ack_num: u8) -> io::Result<&Data> {
        if self.transmits >= MAX_TRANSMISSIONS {
            return Err(Error::new(
                ErrorKind::TimedOut,
                format!(
                    "Retransmission limit of frame #{} exceeded.",
                    self.data.frame_num()
                ),
            ));
        }

        if self.transmits > 0 {
            self.data.set_transmission_fields(ack_num, true);
        }
        self.transmits += 1;
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
    use super::Transmission;
    use crate::frame::Data;

    #[test]
    fn test_new() {
        let data = Data::new(u8::default(), u8::default(), heapless::Vec::new());
        let transmission: Transmission = data.into();
        assert_eq!(transmission.transmits, 0);
        assert!(!transmission.data.is_retransmission());
    }

    #[test]
    fn test_transmit() {
        let data = Data::new(u8::default(), u8::default(), heapless::Vec::new());
        let mut transmission: Transmission = data.into();
        let data = transmission.data_for_transmit(u8::default()).unwrap();
        assert!(!data.is_retransmission());
        assert_eq!(transmission.transmits, 1);
    }

    #[test]
    fn test_retransmit() {
        let data = Data::new(u8::default(), u8::default(), heapless::Vec::new());
        let mut transmission: Transmission = data.into();
        let _transmit = transmission.data_for_transmit(u8::default()).unwrap();
        let retransmit = transmission.data_for_transmit(u8::default()).unwrap();
        assert!(retransmit.is_retransmission());
        assert_eq!(transmission.transmits, 2);
    }
}
