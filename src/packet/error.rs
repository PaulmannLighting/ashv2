use crate::frame::Frame;
use crate::{Code, FrameError, CRC};
use num_traits::FromPrimitive;
use std::array::IntoIter;
use std::fmt::{Display, Formatter};
use std::iter::Chain;

pub const HEADER: u8 = 0xC2;
const SIZE: usize = 5;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Error {
    header: u8,
    version: u8,
    error_code: u8,
    crc: u16,
}

impl Error {
    /// Creates a new ERROR packet.
    #[must_use]
    pub const fn new(header: u8, version: u8, error_code: u8) -> Self {
        Self {
            header,
            version,
            error_code,
            crc: CRC.checksum(&[header, version, error_code]),
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

    #[must_use]
    pub fn iter(&self) -> <&Self as IntoIterator>::IntoIter {
        self.into_iter()
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "ERROR({:#04X}, {:#04X})", self.version, self.error_code)
    }
}

impl Frame for Error {
    fn header(&self) -> u8 {
        self.header
    }

    fn crc(&self) -> u16 {
        self.crc
    }

    fn is_header_valid(&self) -> bool {
        self.header == HEADER
    }

    fn calculate_crc(&self) -> u16 {
        CRC.checksum(&[self.header, self.version, self.error_code])
    }
}

impl IntoIterator for &Error {
    type Item = u8;
    type IntoIter = Chain<
        Chain<Chain<IntoIter<Self::Item, 1>, IntoIter<Self::Item, 1>>, IntoIter<Self::Item, 1>>,
        IntoIter<Self::Item, 2>,
    >;

    fn into_iter(self) -> Self::IntoIter {
        self.header
            .to_be_bytes()
            .into_iter()
            .chain(self.version.to_be_bytes())
            .chain(self.error_code.to_be_bytes())
            .chain(self.crc.to_be_bytes())
    }
}

impl TryFrom<&[u8]> for Error {
    type Error = FrameError;

    fn try_from(buffer: &[u8]) -> Result<Self, Self::Error> {
        if buffer.len() == SIZE {
            Ok(Self {
                header: buffer[0],
                version: buffer[1],
                error_code: buffer[2],
                crc: u16::from_be_bytes([buffer[3], buffer[4]]),
            })
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
    use crate::frame::Frame;
    use crate::Code;

    const ERROR: Error = Error {
        header: 0xC2,
        version: 0x02,
        error_code: 0x51,
        crc: 0xA8BD,
    };

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
    fn test_crc() {
        assert_eq!(ERROR.crc(), 0xA8BD);
    }

    #[test]
    fn test_is_header_valid() {
        assert!(ERROR.is_header_valid());
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
