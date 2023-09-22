use num_derive::{FromPrimitive, ToPrimitive};
use num_traits::FromPrimitive;
use std::fmt::{Display, Formatter};

pub const HEADER: u8 = 0xC1;
pub const VERSION: u8 = 0x02;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RstAck {
    version: u8,
    reset_code: u8,
    crc: u16,
}

impl RstAck {
    #[must_use]
    pub const fn new(version: u8, reset_code: u8, crc: u16) -> Self {
        Self {
            version,
            reset_code,
            crc,
        }
    }

    #[must_use]
    pub const fn version(&self) -> u8 {
        self.version
    }

    #[must_use]
    pub fn reset_code(&self) -> Option<ResetCode> {
        ResetCode::from_u8(self.reset_code)
    }
}

#[derive(Clone, Debug, Eq, PartialEq, FromPrimitive, ToPrimitive)]
pub enum ResetCode {
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

impl Display for ResetCode {
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
