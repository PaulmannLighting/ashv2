use std::fmt::{Display, Formatter};

use crate::error::frame;
use crate::frame::Frame;
use crate::{Code, CRC};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Error {
    header: u8,
    version: u8,
    code: u8,
    crc: u16,
}

impl Error {
    pub const HEADER: u8 = 0xC2;
    pub const SIZE: usize = 5;

    #[must_use]
    pub const fn new(code: u8) -> Self {
        Self {
            header: Self::HEADER,
            version: crate::VERSION,
            code,
            crc: CRC.checksum(&[Self::HEADER, crate::VERSION, code]),
        }
    }

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
    #[must_use]
    pub fn code(&self) -> Option<Code> {
        Code::try_from(self.code).ok()
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "ERROR({:#04X}, {:#04X})", self.version, self.code)
    }
}

impl Frame for Error {
    fn header(&self) -> u8 {
        self.header
    }

    fn crc(&self) -> u16 {
        self.crc
    }

    fn calculate_crc(&self) -> u16 {
        CRC.checksum(&[self.header, self.version, self.code])
    }

    fn bytes(&self) -> impl AsRef<[u8]> {
        let [crc0, crc1] = self.crc.to_be_bytes();
        [self.header, self.version, self.code, crc0, crc1]
    }
}

impl From<Code> for Error {
    fn from(code: Code) -> Self {
        Self::new(code.into())
    }
}

impl TryFrom<&[u8]> for Error {
    type Error = frame::Error;

    fn try_from(buffer: &[u8]) -> Result<Self, Self::Error> {
        if buffer.len() == Self::SIZE {
            Ok(Self {
                header: buffer[0],
                version: buffer[1],
                code: buffer[2],
                crc: u16::from_be_bytes([buffer[3], buffer[4]]),
            })
        } else {
            Err(Self::Error::InvalidBufferSize {
                expected: Self::SIZE,
                found: buffer.len(),
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::frame::Frame;
    use crate::Code;

    use super::Error;

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
    fn test_crc() {
        assert_eq!(ERROR.crc(), 0xA8BD);
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
