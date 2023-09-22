use crate::Code;
use num_traits::FromPrimitive;
use std::fmt::{Display, Formatter};

pub const HEADER: u8 = 0xC2;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Error {
    version: u8,
    error_code: u8,
    crc: u16,
}

impl Error {
    #[must_use]
    pub const fn new(version: u8, error_code: u8, crc: u16) -> Self {
        Self {
            version,
            error_code,
            crc,
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
    /// let error = Error::new(0x01, 0x52, 0xFABD);
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
    /// let error = Error::new(0x01, 0x52, 0xFABD);
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
    /// let error = Error::new(0x01, 0x52, 0xFABD);
    /// assert_eq!(&error.to_string(), "ERROR(0x01, 0x52)");
    /// ```
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "ERROR({:#04x}, {:#04x})", self.version, self.error_code)
    }
}
