use bitflags::bitflags;

use crate::utils::WrappingU3;

/// Negative Acknowledgement (`NAK`) frame header.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct Nak(u8);

bitflags! {
    impl Nak: u8 {
        /// The default NAK header.
        const DEFAULT = 0b1010_0000;
        /// The `nRDY` flag.
        const NOT_READY = 0b0000_1000;
        /// The acknowledgement number mask.
        const ACK_NUM = 0b0000_0111;
    }
}

impl Nak {
    /// Creates a new NAK header.
    #[must_use]
    pub const fn new(ack_num: WrappingU3, n_rdy: bool) -> Self {
        let mut raw = Self::DEFAULT.bits() | (Self::ACK_NUM.bits() & ack_num.as_u8());

        if n_rdy {
            raw |= Self::NOT_READY.bits();
        }

        Self(raw)
    }

    /// Returns the ACK number.
    #[must_use]
    pub const fn ack_num(self) -> WrappingU3 {
        WrappingU3::from_u8_lossy(self.bits() & Self::ACK_NUM.bits())
    }
}

#[cfg(test)]
mod tests {
    use super::Nak;
    use crate::WrappingU3;

    #[test]
    fn test_new() {
        let nak = Nak::new(WrappingU3::from_u8_lossy(3), false);
        assert_eq!(nak.ack_num().as_u8(), 3);
        assert!(!nak.contains(Nak::NOT_READY));
    }

    #[test]
    fn test_new_nrdy() {
        let nak = Nak::new(WrappingU3::from_u8_lossy(3), true);
        assert_eq!(nak.ack_num().as_u8(), 3);
        assert!(nak.contains(Nak::NOT_READY));
    }
}
