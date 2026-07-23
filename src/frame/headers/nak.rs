//! Negative Acknowledgement (`NAK`) frame header.

use bitflags::bitflags;

/// Negative Acknowledgement (`NAK`) frame header.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct Header(u8);

bitflags! {
    impl Header: u8 {
        /// The default NAK header.
        const DEFAULT = 0b1010_0000;

        /// The `nRDY` flag.
        const NOT_READY = 0b0000_1000;

        /// The acknowledgement number mask.
        const ACK_NUM = 0b0000_0111;
    }
}

impl Header {
    /// Creates a new NAK header.
    #[must_use]
    pub const fn new(ack_num: u8, n_rdy: bool) -> Self {
        let mut raw = Self::DEFAULT.bits() | (Self::ACK_NUM.bits() & ack_num);

        if n_rdy {
            raw |= Self::NOT_READY.bits();
        }

        Self(raw)
    }

    /// Returns the ACK number.
    #[must_use]
    pub const fn ack_num(self) -> u8 {
        self.bits() & Self::ACK_NUM.bits()
    }
}

#[cfg(test)]
mod tests {
    use super::Header;

    #[test]
    fn test_new() {
        let nak = Header::new(3, false);
        assert_eq!(nak.ack_num(), 3);
        assert!(!nak.contains(Header::NOT_READY));
    }

    #[test]
    fn test_new_nrdy() {
        let nak = Header::new(3, true);
        assert_eq!(nak.ack_num(), 3);
        assert!(nak.contains(Header::NOT_READY));
    }
}
