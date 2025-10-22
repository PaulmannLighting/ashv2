//! Data frame header.

use bitflags::bitflags;

use crate::utils::WrappingU3;

/// Data frame header.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct Header(u8);

bitflags! {
    impl Header: u8 {
        /// The frame number mask.
        const FRAME_NUM = 0b0111_0000;
        /// The retransmit flag.
        const RETRANSMIT = 0b0000_1000;
        /// The acknowledgement number mask.
        const ACK_NUM = 0b0000_0111;
    }
}

impl Header {
    /// Creates a new data header.
    #[must_use]
    pub const fn new(frame_num: WrappingU3, retransmit: bool, ack_num: WrappingU3) -> Self {
        let mut raw = Self::FRAME_NUM.bits()
            & (frame_num.as_u8() << Self::FRAME_NUM.bits().trailing_zeros())
            | Self::ACK_NUM.bits() & ack_num.as_u8();

        if retransmit {
            raw |= Self::RETRANSMIT.bits();
        }

        Self(raw)
    }

    /// Returns the frame number.
    #[must_use]
    pub const fn frame_num(self) -> WrappingU3 {
        WrappingU3::from_u8_lossy(
            (self.bits() & Self::FRAME_NUM.bits()) >> Self::FRAME_NUM.bits().trailing_zeros(),
        )
    }

    /// Returns the ACK number.
    #[must_use]
    pub const fn ack_num(self) -> WrappingU3 {
        WrappingU3::from_u8_lossy(self.bits() & Self::ACK_NUM.bits())
    }
}

#[cfg(test)]
mod tests {
    use super::Header;

    #[test]
    fn test_offset() {
        assert_eq!(Header::FRAME_NUM.bits().trailing_zeros(), 4);
    }
}
