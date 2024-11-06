use std::fmt::{Display, Formatter};

use num_derive::FromPrimitive;

/// Reset and error codes.
#[derive(Clone, Debug, Eq, Hash, PartialEq, FromPrimitive)]
#[repr(u8)]
pub enum Code {
    /// Reset: Unknown reason
    UnknownReason = 0x00,
    /// Reset: External
    External = 0x01,
    /// Reset: Power-on
    PowerOn = 0x02,
    /// Reset: Watchdog
    Watchdog = 0x03,
    /// Reset: Assert
    Assert = 0x06,
    /// Reset: Boot loader
    Bootloader = 0x09,
    /// Reset: Software
    Software = 0x0B,
    /// Error: Exceeded maximum ACK timeout count
    ExceededMaximumAckTimeoutCount = 0x51,
    /// Chip-specific error reset code
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
