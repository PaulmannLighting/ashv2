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

    /// Returns the protocol version.
    ///
    /// This is statically set to 0x02 (2) for ASHv2.
    ///
    /// # Examples
    /// ```
    /// use ashv2::packet::rst_ack::{RstAck, VERSION};
    ///
    /// let rst_ack = RstAck::new( 0x02, 0x02, 0x9B7B);
    /// assert_eq!(rst_ack.version(), VERSION);
    /// ```
    #[must_use]
    pub const fn version(&self) -> u8 {
        self.version
    }

    /// Returns the reset code.
    ///
    /// # Examples
    /// ```
    /// use ashv2::packet::rst_ack::{ResetCode, RstAck, VERSION};
    ///
    /// let rst_ack = RstAck::new( 0x02, 0x02, 0x9B7B);
    /// assert_eq!(rst_ack.reset_code(), Some(ResetCode::PowerOn));
    /// ```
    #[must_use]
    pub fn reset_code(&self) -> Option<ResetCode> {
        ResetCode::from_u8(self.reset_code)
    }
}

impl Display for RstAck {
    /// Formats the RSTACK as a String..
    ///
    /// # Examples
    /// ```
    /// use ashv2::packet::rst_ack::{ResetCode, RstAck, VERSION};
    ///
    /// let rst_ack = RstAck::new( 0x02, 0x02, 0x9B7B);
    /// assert_eq!(&rst_ack.to_string(), "RSTACK(0x02, 0x02)");
    /// ```
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "RSTACK({:#04x}, {:#04x})", self.version, self.reset_code)
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
