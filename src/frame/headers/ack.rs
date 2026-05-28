//! Acknowledgement (`ACK`) frame header.

use bitflags::bitflags;

use crate::seq::Seq;

/// Acknowledgement (`ACK`) frame header.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct Header(u8);

bitflags! {
    impl Header: u8 {
        /// The default ACK header.
        const DEFAULT = 0b1000_0000;
        /// The `nRDY` flag.
        const NOT_READY = 0b0000_1000;
        /// The acknowledgement number mask.
        const ACK_NUM = 0b0000_0111;
    }
}

impl Header {
    /// Creates a new ACK header.
    #[must_use]
    pub const fn new(ack_num: Seq, n_rdy: bool) -> Self {
        let mut raw = Self::DEFAULT.bits() | Self::ACK_NUM.bits() & ack_num.as_u8();

        if n_rdy {
            raw |= Self::NOT_READY.bits();
        }

        Self(raw)
    }

    /// Returns the ACK number.
    #[must_use]
    pub fn ack_num(self) -> Seq {
        Seq::try_from(self.bits() & Self::ACK_NUM.bits()).expect("Seq always fits.")
    }
}

#[cfg(test)]
mod tests {
    use super::Header;
    use crate::seq::Seq;

    #[test]
    fn test_new() {
        let ack = Header::new(Seq::try_from(3).expect("Seq fits."), false);
        assert_eq!(ack.ack_num().as_u8(), 3);
        assert!(!ack.contains(Header::NOT_READY));
    }

    #[test]
    fn test_new_nrdy() {
        let ack = Header::new(Seq::try_from(3).expect("Seq fits."), true);
        assert_eq!(ack.ack_num().as_u8(), 3);
        assert!(ack.contains(Header::NOT_READY));
    }
}
