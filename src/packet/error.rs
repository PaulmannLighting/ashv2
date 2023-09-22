use crate::{Code, Frame};
use num_traits::FromPrimitive;
use std::fmt::{Display, Formatter};

pub const HEADER: u8 = 0xC2;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Error {
    header: u8,
    version: u8,
    error_code: u8,
    crc: u16,
    flag: u8,
}

impl Error {
    /// Creates a new ERROR packet.
    ///
    /// # Examples
    /// ```
    /// use ashv2::Frame;
    /// use ashv2::packet::error::Error;
    ///
    /// let error = Error::new(0xC2, 0x01, 0x52, 0xFABD, 0x7E);
    /// assert!(error.is_valid());
    /// ```
    #[must_use]
    pub const fn new(header: u8, version: u8, error_code: u8, crc: u16, flag: u8) -> Self {
        Self {
            header,
            version,
            error_code,
            crc,
            flag,
        }
    }

    /// Returns the protocol version.
    ///
    /// This is statically set to 0x02 (2) for ASHv2.
    ///
    /// # Examples
    /// ```
    /// use ashv2::packet::error::Error;
    ///
    /// let error = Error::new(0xC2, 0x01, 0x52, 0xFABD, 0x7E);
    /// assert_eq!(error.version(), 0x01); // By example data, though invalid
    /// ```
    #[must_use]
    pub const fn version(&self) -> u8 {
        self.version
    }

    /// Returns the error code.
    ///
    /// # Examples
    /// ```
    /// use ashv2::packet::error::Error;
    ///
    /// let error = Error::new(0xC2, 0x01, 0x52, 0xFABD, 0x7E);
    /// assert_eq!(error.code(), None); // Invalid error code
    #[must_use]
    pub fn code(&self) -> Option<Code> {
        Code::from_u8(self.error_code)
    }
}

impl Display for Error {
    /// Formats the ERROR as a String.
    ///
    /// # Examples
    /// ```
    /// use ashv2::packet::error::Error;
    ///
    /// let error = Error::new(0xC2, 0x01, 0x52, 0xFABD, 0x7E);
    /// assert_eq!(&error.to_string(), "ERROR(0x01, 0x52)");
    /// ```
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "ERROR({:#04x}, {:#04x})", self.version, self.error_code)
    }
}

impl Frame for Error {
    /// Returns the header.
    ///
    /// # Examples
    /// ```
    /// use ashv2::Frame;
    /// use ashv2::packet::error::{Error, HEADER};
    ///
    /// let error = Error::new(0xC2, 0x01, 0x52, 0xFABD, 0x7E);
    /// assert_eq!(error.header(), 0xC2);
    /// ```
    fn header(&self) -> u8 {
        self.header
    }

    /// Returns the payload.
    ///
    /// # Examples
    /// ```
    /// use ashv2::Frame;
    /// use ashv2::packet::error::{Error, HEADER};
    ///
    /// let error = Error::new(0xC2, 0x01, 0x52, 0xFABD, 0x7E);
    /// assert_eq!(error.payload(), Some(vec![0x01, 0x52]));
    /// ```
    fn payload(&self) -> Option<Vec<u8>> {
        Some(vec![self.version, self.error_code])
    }

    /// Returns the CRC checksum.
    ///
    /// # Examples
    /// ```
    /// use ashv2::Frame;
    /// use ashv2::packet::error::{Error, HEADER};
    ///
    /// let error = Error::new(0xC2, 0x01, 0x52, 0xFABD, 0x7E);
    /// assert_eq!(error.crc(), 0xFABD);
    /// ```
    fn crc(&self) -> u16 {
        self.crc
    }

    /// Returns the flag byte.
    ///
    /// # Examples
    /// ```
    /// use ashv2::Frame;
    /// use ashv2::packet::error::{Error, HEADER};
    ///
    /// let error = Error::new(0xC2, 0x01, 0x52, 0xFABD, 0x7E);
    /// assert_eq!(error.flag(), 0x7E);
    /// ```
    fn flag(&self) -> u8 {
        self.flag
    }

    /// Determines whether the header is valid.
    ///
    /// # Examples
    /// ```
    /// use ashv2::Frame;
    /// use ashv2::packet::error::{Error, HEADER};
    ///
    /// let error = Error::new(0xC2, 0x01, 0x52, 0xFABD, 0x7E);
    /// assert!(error.is_header_valid());
    /// ```
    fn is_header_valid(&self) -> bool {
        self.header == HEADER
    }
}
