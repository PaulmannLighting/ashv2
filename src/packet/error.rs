use crate::{Code, Frame};
use num_traits::FromPrimitive;
use std::fmt::{Display, Formatter};

pub const HEADER: u8 = 0xC2;
pub const SIZE: usize = 6;

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
    /// This is statically set to `0x02` (2) for `ASHv2`.
    #[must_use]
    pub const fn version(&self) -> u8 {
        self.version
    }

    /// Returns the error code.
    #[must_use]
    pub fn code(&self) -> Option<Code> {
        Code::from_u8(self.error_code)
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "ERROR({:#04x}, {:#04x})", self.version, self.error_code)
    }
}

impl Frame for Error {
    fn header(&self) -> u8 {
        self.header
    }

    fn payload(&self) -> Option<Vec<u8>> {
        Some(vec![self.version, self.error_code])
    }

    fn crc(&self) -> u16 {
        self.crc
    }

    fn flag(&self) -> u8 {
        self.flag
    }

    fn is_header_valid(&self) -> bool {
        self.header == HEADER
    }
}

impl TryFrom<&[u8]> for Error {
    type Error = crate::packet::Error;

    fn try_from(buffer: &[u8]) -> Result<Self, Self::Error> {
        if buffer.len() == SIZE {
            Ok(Self::new(
                buffer[0],
                buffer[1],
                buffer[2],
                u16::from_be_bytes([buffer[3], buffer[4]]),
                buffer[5],
            ))
        } else {
            Err(Self::Error::InvalidBufferSize {
                expected: SIZE,
                found: buffer.len(),
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Error;
    use crate::{Code, Frame};

    const ERROR: Error = Error::new(0xC2, 0x02, 0x51, 0xA8BD, 0x7E);

    #[test]
    fn test_is_valid() {
        assert!(ERROR.is_valid());
    }

    #[test]
    fn test_version() {
        assert_eq!(ERROR.version(), 2);
    }

    #[test]
    fn test_code() {
        assert_eq!(ERROR.code(), Some(Code::ExceededMaximumAckTimeoutCount));
    }

    #[test]
    fn test_to_string() {
        assert_eq!(&ERROR.to_string(), "ERROR(0x02, 0x51)");
    }

    #[test]
    fn test_header() {
        assert_eq!(ERROR.header(), 0xC2);
    }

    #[test]
    fn test_payload() {
        assert_eq!(ERROR.payload(), Some(vec![0x02, 0x51]));
    }

    #[test]
    fn test_crc() {
        assert_eq!(ERROR.crc(), 0xA8BD);
    }

    #[test]
    fn test_flag() {
        assert_eq!(ERROR.flag(), 0x7E);
    }

    #[test]
    fn test_is_header_valid() {
        assert!(ERROR.is_header_valid());
    }

    #[test]
    fn test_from_buffer() {
        let buffer: Vec<u8> = vec![0xC2, 0x02, 0x51, 0xA8, 0xBD, 0x7E];
        assert_eq!(Error::try_from(buffer.as_slice()), Ok(ERROR));
    }
}
