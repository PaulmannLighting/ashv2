use bitflags::bitflags;

const FRAME_NUM_OFFSET: u8 = 4;

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct Data(u8);

bitflags! {
    impl Data: u8 {
        const DEFAULT = 0b0000_0000;
        const FRAME_NUM = 0b0111_0000;
        const RETRANSMIT = 0b0000_1000;
        const ACK_NUM = 0b0000_0111;
    }
}

impl Data {
    pub fn new(frame_num: u8, retransmit: bool, ack_num: u8) -> Self {
        let mut ack = Self::DEFAULT;
        ack |= Self::FRAME_NUM & Self::from_bits_retain(frame_num << FRAME_NUM_OFFSET);
        ack.set(Self::RETRANSMIT, retransmit);
        ack |= Self::ACK_NUM & Self::from_bits_retain(ack_num);
        ack
    }

    /// Returns the frame number.
    pub const fn frame_num(self) -> u8 {
        (self.bits() & Self::FRAME_NUM.bits()) >> FRAME_NUM_OFFSET
    }

    /// Returns the ACK number.
    pub const fn ack_num(self) -> u8 {
        self.bits() & Self::ACK_NUM.bits()
    }
}
