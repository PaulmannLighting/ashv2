use num_derive::{FromPrimitive, ToPrimitive};
use num_traits::ToPrimitive;

/// Available baud rates that the NCP can operate on.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, FromPrimitive, ToPrimitive)]
pub enum BaudRate {
    RstCts = 115_200,
    XOnXOff = 57_600,
}

impl From<BaudRate> for u32 {
    fn from(baud_rate: BaudRate) -> Self {
        baud_rate
            .to_u32()
            .expect("Baud rate should always be convertible to u32.")
    }
}

#[cfg(feature = "clap")]
impl clap::ValueEnum for BaudRate {
    fn value_variants<'a>() -> &'a [Self] {
        &[Self::RstCts, Self::XOnXOff]
    }

    fn to_possible_value(&self) -> Option<clap::builder::PossibleValue> {
        Some(match self {
            Self::RstCts => clap::builder::PossibleValue::new("RST/CTS")
                .alias("RST_CTS")
                .alias("RstCts"),
            Self::XOnXOff => clap::builder::PossibleValue::new("XON/XOFF")
                .alias("XON_XOFF")
                .alias("XOnXOff"),
        })
    }
}
