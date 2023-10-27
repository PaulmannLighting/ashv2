use num_derive::{FromPrimitive, ToPrimitive};
use num_traits::ToPrimitive;
use std::fmt::{Display, Formatter};

#[derive(Clone, Debug, Eq, PartialEq, FromPrimitive, ToPrimitive)]
pub enum Code {
    UnknownReason = 0x00,
    External = 0x01,
    PowerOn = 0x02,
    Watchdog = 0x03,
    Assert = 0x06,
    Bootloader = 0x09,
    Software = 0x0B,
    ExceededMaximumAckTimeoutCount = 0x51,
    ChipSpecific = 0x80,
}

impl Display for Code {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnknownReason => write!(f, "Reset: Unknown reason"),
            Self::External => write!(f, "Reset: External"),
            Self::PowerOn => write!(f, "Reset: Power-on"),
            Self::Watchdog => write!(f, "Reset: Watchdog"),
            Self::Assert => write!(f, "Reset: Assert"),
            Self::Bootloader => write!(f, "Reset: Boot loader"),
            Self::Software => write!(f, "Reset: Software"),
            Self::ExceededMaximumAckTimeoutCount => {
                write!(f, "Error: Exceeded maximum ACK timeout count")
            }
            Self::ChipSpecific => write!(f, "Chip-specific error reset code"),
        }
    }
}

impl From<Code> for u8 {
    fn from(code: Code) -> Self {
        code.to_u8().expect("Could not convert Code to u8.")
    }
}
