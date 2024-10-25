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
    pub fn new(ack_num: WrappingU3, n_rdy: bool) -> Self {
        let mut ack = Self::DEFAULT;
        ack |= Self::ACK_NUM & Self::from_bits_retain(ack_num.as_u8());
        ack.set(Self::NOT_READY, n_rdy);
        ack
    }

    /// Returns the ACK number.
    #[must_use]
    pub const fn ack_num(self) -> WrappingU3 {
        WrappingU3::from_u8_lossy(self.bits() & Self::ACK_NUM.bits())
    }
}
