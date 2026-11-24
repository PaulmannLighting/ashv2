//! Error frame implementation.

use core::fmt::{Display, Formatter, LowerHex, UpperHex};
use std::io;
use std::iter::{Chain, Peekable};
use std::vec::Drain;

use num_traits::FromPrimitive;

use super::{read_byte, read_crc};
use crate::code::Code;
use crate::hex_slice::HexSlice;
use crate::validate::{CRC, Validate};

/// Error frame.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct Error {
    header: u8,
    version: u8,
    code: u8,
    crc: u16,
}

impl Error {
    /// Constant header value for `ERROR` frames.
    pub const HEADER: u8 = 0xC2;

    /// Returns the protocol version reported by the peer.
    ///
    /// Use [`Self::is_ash_v2`] to check whether the reported version is supported.
    #[must_use]
    pub const fn version(self) -> u8 {
        self.version
    }

    /// Verifies that this is indeed `ASHv2`.
    #[must_use]
    pub const fn is_ash_v2(self) -> bool {
        self.version() == crate::VERSION
    }

    /// Returns the error code.
    ///
    /// # Errors
    ///
    /// Returns an error if the error code is invalid.
    pub fn code(self) -> Result<Code, u8> {
        Code::from_u8(self.code).ok_or(self.code)
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self.code() {
            Ok(code) => write!(f, "ERROR({:#04X}, {code})", self.version()),
            Err(code) => write!(f, "ERROR({:#04X}, {code:#04X})", self.version()),
        }
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

impl TryFrom<Peekable<Drain<'_, u8>>> for Error {
    type Error = io::Error;

    fn try_from(mut buffer: Peekable<Drain<'_, u8>>) -> io::Result<Self> {
        let header = read_byte(&mut buffer)?;
        let version = read_byte(&mut buffer)?;
        let code = read_byte(&mut buffer)?;
        let crc = read_crc(&mut buffer)?;

        Ok(Self {
            header,
            version,
            code,
            crc,
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
        assert_eq!(
            &ERROR.to_string(),
            "ERROR(0x02, Error: Exceeded maximum ACK timeout count)"
        );
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
        assert!(ERROR.validate().is_ok());
    }

    #[test]
    fn test_from_buffer() {
        let mut buffer: Vec<u8> = vec![0xC2, 0x02, 0x51, 0xA8, 0xBD];
        assert_eq!(
            Error::try_from(buffer.drain(..).peekable())
                .expect("Reference frame should be a valid ERROR."),
            ERROR
        );
    }
}
