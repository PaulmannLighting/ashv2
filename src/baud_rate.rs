use num_derive::{FromPrimitive, ToPrimitive};
use num_traits::{FromPrimitive, ToPrimitive};
use std::str::FromStr;
use strum::IntoStaticStr;

#[derive(
    Clone, Debug, Eq, Ord, PartialEq, PartialOrd, FromPrimitive, ToPrimitive, IntoStaticStr,
)]
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

impl FromStr for BaudRate {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::from_u32(s.parse::<u32>().map_err(|error| error.to_string())?)
            .ok_or_else(|| "unsupported baud rate".to_string())
    }
}
