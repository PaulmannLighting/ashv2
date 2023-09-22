use crate::code::Code;
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
    /// use ashv2::packet::rst_ack::RstAck;
    /// use ashv2::Code;
    ///
    /// let rst_ack = RstAck::new( 0x02, 0x02, 0x9B7B);
    /// assert_eq!(rst_ack.code(), Some(Code::PowerOn));
    /// ```
    #[must_use]
    pub fn code(&self) -> Option<Code> {
        Code::from_u8(self.reset_code)
    }
}

impl Display for RstAck {
    /// Formats the RSTACK as a String.
    ///
    /// # Examples
    /// ```
    /// use ashv2::packet::rst_ack::RstAck;
    ///
    /// let rst_ack = RstAck::new( 0x02, 0x02, 0x9B7B);
    /// assert_eq!(&rst_ack.to_string(), "RSTACK(0x02, 0x02)");
    /// ```
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "RSTACK({:#04x}, {:#04x})", self.version, self.reset_code)
    }
}
