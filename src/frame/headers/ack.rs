use bitflags::bitflags;

use crate::utils::WrappingU3;

/// Acknowledgement (`ACK`) frame header.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct Ack(u8);

bitflags! {
    impl Ack: u8 {
        /// The default ACK header.
        const DEFAULT = 0b1000_0000;
        /// The `nRDY` flag.
        const NOT_READY = 0b0000_1000;
        /// The acknowledgement number mask.
        const ACK_NUM = 0b0000_0111;
    }
}

impl Ack {
    /// Creates a new ACK header.
    #[must_use]
    pub const fn new(ack_num: WrappingU3, n_rdy: bool) -> Self {
        let mut raw = Self::DEFAULT.bits() | Self::ACK_NUM.bits() & ack_num.as_u8();

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
    use super::Ack;
    use crate::utils::WrappingU3;

    #[test]
    fn test_new() {
        let ack = Ack::new(WrappingU3::from_u8_lossy(3), false);
        assert_eq!(ack.ack_num().as_u8(), 3);
        assert!(!ack.contains(Ack::NOT_READY));
    }

    #[test]
    fn test_new_nrdy() {
        let ack = Ack::new(WrappingU3::from_u8_lossy(3), true);
        assert_eq!(ack.ack_num().as_u8(), 3);
        assert!(ack.contains(Ack::NOT_READY));
    }
}
