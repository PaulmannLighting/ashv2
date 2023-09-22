use crate::code::Code;
use crate::Frame;
use num_traits::FromPrimitive;
use std::fmt::{Display, Formatter};

pub const HEADER: u8 = 0xC1;
pub const VERSION: u8 = 0x02;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RstAck {
    header: u8,
    version: u8,
    reset_code: u8,
    crc: u16,
    flag: u8,
}

impl RstAck {
    /// Creates a new RSTACK packet.
    ///
    /// # Examples
    /// ```
    /// use ashv2::Frame;
    /// use ashv2::packet::rst_ack::RstAck;
    ///
    /// let rst_ack = RstAck::new(0xC1, 0x02, 0x02, 0x9B7B, 0x7E);
    /// assert!(rst_ack.is_valid());
    /// ```
    #[must_use]
    pub const fn new(header: u8, version: u8, reset_code: u8, crc: u16, flag: u8) -> Self {
        Self {
            header,
            version,
            reset_code,
            crc,
            flag,
        }
    }

    /// Returns the protocol version.
    ///
    /// This is statically set to `0x02` (2) for `ASHv2`.
    ///
    /// # Examples
    /// ```
    /// use ashv2::packet::rst_ack::{RstAck, VERSION};
    ///
    /// let rst_ack = RstAck::new(0xC1, 0x02, 0x02, 0x9B7B, 0x7E);
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
    /// let rst_ack = RstAck::new(0xC1, 0x02, 0x02, 0x9B7B, 0x7E);
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
    /// let rst_ack = RstAck::new(0xC1, 0x02, 0x02, 0x9B7B, 0x7E);
    /// assert_eq!(&rst_ack.to_string(), "RSTACK(0x02, 0x02)");
    /// ```
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "RSTACK({:#04x}, {:#04x})", self.version, self.reset_code)
    }
}

impl Frame for RstAck {
    /// Returns the header.
    ///
    /// # Examples
    /// ```
    /// use ashv2::Frame;
    /// use ashv2::packet::rst_ack::RstAck;
    ///
    /// let rst_ack = RstAck::new(0xC1, 0x02, 0x02, 0x9B7B, 0x7E);
    /// assert_eq!(rst_ack.header(), 0xC1);
    /// ```
    fn header(&self) -> u8 {
        self.header
    }

    /// Returns the payload.
    ///
    /// # Examples
    /// ```
    /// use ashv2::Frame;
    /// use ashv2::packet::rst_ack::RstAck;
    ///
    /// let rst_ack = RstAck::new(0xC1, 0x02, 0x02, 0x9B7B, 0x7E);
    /// assert_eq!(rst_ack.payload(), Some(vec![0x02, 0x02]));
    /// ```
    fn payload(&self) -> Option<Vec<u8>> {
        Some(vec![self.version, self.reset_code])
    }

    /// Returns the CRC checksum.
    ///
    /// # Examples
    /// ```
    /// use ashv2::Frame;
    /// use ashv2::packet::rst_ack::RstAck;
    ///
    /// let rst_ack = RstAck::new(0xC1, 0x02, 0x02, 0x9B7B, 0x7E);
    /// assert_eq!(rst_ack.crc(), 0x9B7B);
    /// ```
    fn crc(&self) -> u16 {
        self.crc
    }

    /// Returns the flag byte.
    ///
    /// # Examples
    /// ```
    /// use ashv2::Frame;
    /// use ashv2::packet::rst_ack::RstAck;
    ///
    /// let rst_ack = RstAck::new(0xC1, 0x02, 0x02, 0x9B7B, 0x7E);
    /// assert_eq!(rst_ack.flag(), 0x7E);
    /// ```
    fn flag(&self) -> u8 {
        self.flag
    }

    /// Determines whether the header is valid.
    ///
    /// # Examples
    /// ```
    /// use ashv2::Frame;
    /// use ashv2::packet::rst_ack::RstAck;
    ///
    /// let rst_ack = RstAck::new(0xC1, 0x02, 0x02, 0x9B7B, 0x7E);
    /// assert!(rst_ack.is_header_valid());
    /// ```
    fn is_header_valid(&self) -> bool {
        self.header == HEADER
    }
}
