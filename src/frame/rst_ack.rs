//! Reset acknowledgment (`RST_ACK`) frame implementation.

use core::fmt::{Display, Formatter, LowerHex, UpperHex};
use std::io::{self, Error};
use std::iter::Chain;

use num_traits::FromPrimitive;

use crate::VERSION;
use crate::code::Code;
use crate::utils::HexSlice;
use crate::validate::{CRC, Validate};

/// A reset acknowledgment (`RST_ACK`) frame.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RstAck {
    header: u8,
    version: u8,
    reset_code: u8,
    crc: u16,
}

impl RstAck {
    /// Constant header value for `RST_ACK` frames.
    pub const HEADER: u8 = 0xC1;

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
        self.version == VERSION
    }

    /// Returns the reset code.
    ///
    /// # Errors
    ///
    /// Returns an error if the reset code is invalid.
    pub fn code(&self) -> Result<Code, u8> {
        Code::from_u8(self.reset_code).ok_or(self.reset_code)
    }
}

impl Display for RstAck {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self.code() {
            Ok(code) => write!(f, "RSTACK({:#04X}, {code})", self.version),
            Err(code) => write!(f, "RSTACK({:#04X}, {code:#04X})", self.version),
        }
    }
}

impl Validate for RstAck {
    fn crc(&self) -> u16 {
        self.crc
    }

    fn calculate_crc(&self) -> u16 {
        CRC.checksum(&[self.header, self.version, self.reset_code])
    }
}

impl IntoIterator for RstAck {
    type Item = u8;
    type IntoIter = Chain<<[u8; 3] as IntoIterator>::IntoIter, <[u8; 2] as IntoIterator>::IntoIter>;

    fn into_iter(self) -> Self::IntoIter {
        [self.header, self.version, self.reset_code]
            .into_iter()
            .chain(self.crc.to_be_bytes())
    }
}

impl TryFrom<&[u8]> for RstAck {
    type Error = Error;

    fn try_from(buffer: &[u8]) -> io::Result<Self> {
        let [header, version, reset_code, crc0, crc1] = buffer else {
            return Err(Error::other("Invalid RST_ACK frame size."));
        };

        Ok(Self {
            header: *header,
            version: *version,
            reset_code: *reset_code,
            crc: u16::from_be_bytes([*crc0, *crc1]),
        })
    }
}

impl UpperHex for RstAck {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "RstAck {{ header: ")?;
        UpperHex::fmt(&self.header, f)?;
        write!(f, ", version: ")?;
        UpperHex::fmt(&self.version, f)?;
        write!(f, ", reset_code: ")?;
        UpperHex::fmt(&self.reset_code, f)?;
        write!(f, ", crc: ")?;
        UpperHex::fmt(&HexSlice::new(&self.crc.to_be_bytes()), f)?;
        write!(f, " }}")
    }
}

impl LowerHex for RstAck {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "RstAck {{ header: ")?;
        LowerHex::fmt(&self.header, f)?;
        write!(f, ", version: ")?;
        LowerHex::fmt(&self.version, f)?;
        write!(f, ", reset_code: ")?;
        LowerHex::fmt(&self.reset_code, f)?;
        write!(f, ", crc: ")?;
        LowerHex::fmt(&HexSlice::new(&self.crc.to_be_bytes()), f)?;
        write!(f, " }}")
    }
}

#[cfg(test)]
mod tests {
    use super::RstAck;
    use crate::code::Code;
    use crate::validate::Validate;

    const RST_ACK: RstAck = RstAck {
        header: 0xC1,
        version: 0x02,
        reset_code: 0x02,
        crc: 0x9B7B,
    };

    #[test]
    fn test_version() {
        assert_eq!(RST_ACK.version(), 0x02);
    }

    #[test]
    fn test_code() {
        assert_eq!(RST_ACK.code(), Ok(Code::PowerOn));
    }

    #[test]
    fn test_to_string() {
        assert_eq!(&RST_ACK.to_string(), "RSTACK(0x02, Reset: Power-on)");
    }

    #[test]
    fn test_header() {
        assert_eq!(RST_ACK.header, 0xC1);
    }

    #[test]
    fn test_crc() {
        assert_eq!(RST_ACK.crc(), 0x9B7B);
    }

    #[test]
    fn test_is_crc_valid() {
        assert!(RST_ACK.validate().is_ok());
    }

    #[test]
    fn test_from_buffer() {
        let buffer: Vec<u8> = vec![0xC1, 0x02, 0x02, 0x9B, 0x7B];
        assert_eq!(
            RstAck::try_from(buffer.as_slice()).expect("Reference frame should be a valid RSTACK"),
            RST_ACK
        );
    }
}
