use num_derive::{FromPrimitive, ToPrimitive};
use num_traits::ToPrimitive;

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, FromPrimitive, ToPrimitive)]
pub enum BaudRate {
    RstCts = 115_200,
    XOnXOff = 57_600,
}

impl From<BaudRate> for u32 {
    fn from(baud_rate: BaudRate) -> Self {
        baud_rate
            .to_u32()
            .expect("could not convert baud rate to u32")
    }
}
