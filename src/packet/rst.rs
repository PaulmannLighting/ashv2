use crate::Frame;
use std::fmt::{Display, Formatter};

pub const HEADER: u8 = 0xC0;
pub const CRC: u16 = 0x38BC;

/// Requests the NCP to perform a software reset (valid even if the NCP is in the FAILED state).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Rst {
    header: u8,
    crc: u16,
    flag: u8,
}

impl Rst {
    /// Creates a new RST packet.
    ///
    /// # Examples
    /// ```
    /// use ashv2::Frame;
    /// use ashv2::packet::rst::Rst;
    ///
    /// let rst = Rst::new(0xC0, 0x38BC, 0x7E);
    /// assert!(rst.is_valid());
    /// ```
    #[must_use]
    pub const fn new(header: u8, crc: u16, flag: u8) -> Self {
        Self { header, crc, flag }
    }
}

impl Display for Rst {
    /// Display the RST packet
    ///
    /// # Examples
    /// ```
    /// use ashv2::packet::rst::Rst;
    ///
    /// let rst = Rst::new(0xC0, 0x38BC, 0x7E);
    /// assert_eq!(&rst.to_string(), "RST()");
    /// ```
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "RST()")
    }
}

impl Frame for Rst {
    /// Returns the header.
    ///
    /// # Examples
    /// ```
    /// use ashv2::Frame;
    /// use ashv2::packet::rst::Rst;
    ///
    /// let rst = Rst::new(0xC0, 0x38BC, 0x7E);
    /// assert_eq!(rst.header(), 0xC0);
    /// ```
    fn header(&self) -> u8 {
        self.header
    }

    /// Returns the payload.
    ///
    /// # Examples
    /// ```
    /// use ashv2::Frame;
    /// use ashv2::packet::rst::Rst;
    ///
    /// let rst = Rst::new(0xC0, 0x38BC, 0x7E);
    /// assert_eq!(rst.payload(), None);
    /// ```
    fn payload(&self) -> Option<Vec<u8>> {
        None
    }

    /// Returns the CRC checksum.
    ///
    /// # Examples
    /// ```
    /// use ashv2::Frame;
    /// use ashv2::packet::rst::Rst;
    ///
    /// let rst = Rst::new(0xC0, 0x38BC, 0x7E);
    /// assert_eq!(rst.crc(), 0x38BC);
    /// ```
    fn crc(&self) -> u16 {
        self.crc
    }

    /// Returns the flag byte.
    ///
    /// # Examples
    /// ```
    /// use ashv2::Frame;
    /// use ashv2::packet::rst::Rst;
    ///
    /// let rst = Rst::new(0xC0, 0x38BC, 0x7E);
    /// assert_eq!(rst.flag(), 0x7E);
    /// ```
    fn flag(&self) -> u8 {
        self.flag
    }

    /// Determines whether the header is valid.
    ///
    /// # Examples
    /// ```
    /// use ashv2::Frame;
    /// use ashv2::packet::rst::Rst;
    ///
    /// let rst = Rst::new(0xC0, 0x38BC, 0x7E);
    /// assert!(rst.is_header_valid());
    /// ```
    fn is_header_valid(&self) -> bool {
        self.header == HEADER
    }
}
