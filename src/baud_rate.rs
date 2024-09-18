/// Available baud rates that the NCP can operate on.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
#[repr(u32)]
pub enum BaudRate {
    /// Baud rate for hardware flow control using RTS/CTS.
    RstCts = 115_200,
    /// Baud rate for software flow control using XON/XOFF.
    XOnXOff = 57_600,
}

impl From<BaudRate> for u32 {
    fn from(baud_rate: BaudRate) -> Self {
        baud_rate as Self
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
