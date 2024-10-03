use std::fmt::{Display, Formatter};

/// Reset and error codes.
#[derive(Clone, Debug, Eq, PartialEq)]
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

impl From<Code> for u8 {
    fn from(code: Code) -> Self {
        code as Self
    }
}

impl TryFrom<u8> for Code {
    type Error = u8;

    fn try_from(byte: u8) -> Result<Self, Self::Error> {
        match byte {
            byte if byte == Self::UnknownReason as u8 => Ok(Self::UnknownReason),
            byte if byte == Self::External as u8 => Ok(Self::External),
            byte if byte == Self::PowerOn as u8 => Ok(Self::PowerOn),
            byte if byte == Self::Watchdog as u8 => Ok(Self::Watchdog),
            byte if byte == Self::Assert as u8 => Ok(Self::Assert),
            byte if byte == Self::Bootloader as u8 => Ok(Self::Bootloader),
            byte if byte == Self::Software as u8 => Ok(Self::Software),
            byte if byte == Self::ExceededMaximumAckTimeoutCount as u8 => {
                Ok(Self::ExceededMaximumAckTimeoutCount)
            }
            byte if byte == Self::ChipSpecific as u8 => Ok(Self::ChipSpecific),
            other => Err(other),
        }
    }
}
