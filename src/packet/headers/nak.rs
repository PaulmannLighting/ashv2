use crate::wrapping_u3::WrappingU3;
use bitflags::bitflags;

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct Nak(u8);

bitflags! {
    impl Nak: u8 {
        const DEFAULT = 0b1010_0000;
        const RESERVED = 0b0001_0000;
        const NOT_READY = 0b0000_1000;
        const ACK_NUM = 0b0000_0111;
    }
}

impl Nak {
    /// Creates a new NAK header.
    #[must_use]
    pub fn new(ack_num: WrappingU3, n_rdy: bool, reserved: bool) -> Self {
        let mut ack = Self::DEFAULT;
        ack |= Self::ACK_NUM & Self::from_bits_retain(ack_num.as_u8());
        ack.set(Self::NOT_READY, n_rdy);
        ack.set(Self::RESERVED, reserved);
        ack
    }

    /// Returns the ACK number.
    #[must_use]
    pub const fn ack_num(self) -> WrappingU3 {
        WrappingU3::from_u8_lossy(self.bits() & Self::ACK_NUM.bits())
    }
}
