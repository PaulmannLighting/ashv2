//! Error frame implementation.

use core::fmt::{Display, Formatter, LowerHex, UpperHex};
use std::io::{self, ErrorKind};
use std::iter::Chain;

use num_traits::FromPrimitive;

use crate::code::Code;
use crate::utils::HexSlice;
use crate::validate::{CRC, Validate};

/// Error frame.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Error {
    header: u8,
    version: u8,
    code: u8,
    crc: u16,
}

impl Error {
    /// Constant header value for `ERROR` frames.
    pub const HEADER: u8 = 0xC2;

    /// The size of the `ERROR` frame in bytes.
    pub const SIZE: usize = 5;

    /// Returns the protocol version.
    ///
    /// This is statically set to `0x02` (2) for `ASHv2`.
    #[must_use]
    pub const fn version(&self) -> u8 {
        self.version
    }

    /// Verifies that this is indeed `ASHv2`.
    #[must_use]
    pub const fn is_ash_v2(&self) -> bool {
        self.version == crate::VERSION
    }

    /// Returns the error code.
    ///
    /// # Errors
    ///
    /// Returns an error if the error code is invalid.
    pub fn code(&self) -> Result<Code, u8> {
        Code::from_u8(self.code).ok_or(self.code)
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "ERROR({:#04X}, {:#04X})", self.version, self.code)
    }
}

impl Validate for Error {
    fn crc(&self) -> u16 {
        self.crc
    }

    fn calculate_crc(&self) -> u16 {
        CRC.checksum(&[self.header, self.version, self.code])
    }
}

impl IntoIterator for Error {
    type Item = u8;
    type IntoIter = Chain<<[u8; 3] as IntoIterator>::IntoIter, <[u8; 2] as IntoIterator>::IntoIter>;

    fn into_iter(self) -> Self::IntoIter {
        [self.header, self.version, self.code]
            .into_iter()
            .chain(self.crc.to_be_bytes())
    }
}

impl TryFrom<&[u8]> for Error {
    type Error = io::Error;

    fn try_from(buffer: &[u8]) -> io::Result<Self> {
        let [header, version, code, crc0, crc1] = buffer else {
            return Err(if buffer.len() < Self::SIZE {
                io::Error::new(ErrorKind::UnexpectedEof, "Too few bytes for ERROR.")
            } else {
                io::Error::new(ErrorKind::OutOfMemory, "Too many bytes for ERROR.")
            });
        };

        Ok(Self {
            header: *header,
            version: *version,
            code: *code,
            crc: u16::from_be_bytes([*crc0, *crc1]),
        })
    }
}

impl UpperHex for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "Error {{ header: ")?;
        UpperHex::fmt(&self.header, f)?;
        write!(f, ", version: ")?;
        UpperHex::fmt(&self.version, f)?;
        write!(f, ", code: ")?;
        UpperHex::fmt(&self.code, f)?;
        write!(f, ", crc: ")?;
        UpperHex::fmt(&HexSlice::new(&self.crc.to_be_bytes()), f)?;
        write!(f, " }}")
    }
}

impl LowerHex for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "Error {{ header: ")?;
        LowerHex::fmt(&self.header, f)?;
        write!(f, ", version: ")?;
        LowerHex::fmt(&self.version, f)?;
        write!(f, ", code: ")?;
        LowerHex::fmt(&self.code, f)?;
        write!(f, ", crc: ")?;
        LowerHex::fmt(&HexSlice::new(&self.crc.to_be_bytes()), f)?;
        write!(f, " }}")
    }
}

#[cfg(test)]
mod tests {
    use super::Error;
    use crate::code::Code;
    use crate::validate::Validate;

    const ERROR: Error = Error {
        header: 0xC2,
        version: 0x02,
        code: 0x51,
        crc: 0xA8BD,
    };

    #[test]
    fn test_version() {
        assert_eq!(ERROR.version(), 2);
    }

    #[test]
    fn test_code() {
        assert_eq!(ERROR.code(), Ok(Code::ExceededMaximumAckTimeoutCount));
    }

    #[test]
    fn test_to_string() {
        assert_eq!(&ERROR.to_string(), "ERROR(0x02, 0x51)");
    }

    #[test]
    fn test_header() {
        assert_eq!(ERROR.header, 0xC2);
    }

    #[test]
    fn test_crc() {
        assert_eq!(ERROR.crc(), 0xA8BD);
    }

    #[test]
    fn test_is_crc_valid() {
        assert!(ERROR.is_crc_valid());
    }

    #[test]
    fn test_from_buffer() {
        let buffer: Vec<u8> = vec![0xC2, 0x02, 0x51, 0xA8, 0xBD];
        assert_eq!(
            Error::try_from(buffer.as_slice()).expect("Reference frame should be a valid ERROR."),
            ERROR
        );
    }
}
