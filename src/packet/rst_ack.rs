use crate::code::Code;
use crate::{Frame, CRC};
use num_traits::FromPrimitive;
use std::fmt::{Display, Formatter};

pub const HEADER: u8 = 0xC1;
pub const SIZE: usize = 5;
pub const VERSION: u8 = 0x02;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RstAck {
    header: u8,
    version: u8,
    reset_code: u8,
    crc: u16,
}

impl RstAck {
    /// Creates a new RSTACK packet.
    #[must_use]
    pub const fn new(header: u8, version: u8, reset_code: u8, crc: u16) -> Self {
        Self {
            header,
            version,
            reset_code,
            crc,
        }
    }

    /// Returns the protocol version.
    ///
    /// This is statically set to `0x02` (2) for `ASHv2`.
    #[must_use]
    pub const fn version(&self) -> u8 {
        self.version
    }

    /// Returns the reset code.
    #[must_use]
    pub fn code(&self) -> Option<Code> {
        Code::from_u8(self.reset_code)
    }
}

impl Display for RstAck {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "RSTACK({:#04x}, {:#04x})", self.version, self.reset_code)
    }
}

impl Frame for RstAck {
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
        CRC.checksum(&[self.header, self.version, self.reset_code])
    }
}

impl From<&RstAck> for Vec<u8> {
    fn from(rst_ack: &RstAck) -> Self {
        let mut bytes = Vec::with_capacity(SIZE);
        bytes.extend_from_slice(&[rst_ack.header, rst_ack.version, rst_ack.reset_code]);
        bytes.extend_from_slice(&rst_ack.crc.to_be_bytes());
        bytes
    }
}

impl TryFrom<&[u8]> for RstAck {
    type Error = crate::Error;

    fn try_from(buffer: &[u8]) -> Result<Self, Self::Error> {
        if buffer.len() == SIZE {
            Ok(Self::new(
                buffer[0],
                buffer[1],
                buffer[2],
                u16::from_be_bytes([buffer[3], buffer[4]]),
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
    use super::RstAck;
    use crate::{Code, Frame};

    const RST_ACK: RstAck = RstAck::new(0xC1, 0x02, 0x02, 0x9B7B);

    #[test]
    fn test_is_valid() {
        assert!(RST_ACK.is_valid());
    }

    #[test]
    fn test_version() {
        assert_eq!(RST_ACK.version(), 0x02);
    }

    #[test]
    fn test_code() {
        assert_eq!(RST_ACK.code(), Some(Code::PowerOn));
    }

    #[test]
    fn test_to_string() {
        assert_eq!(&RST_ACK.to_string(), "RSTACK(0x02, 0x02)");
    }

    #[test]
    fn test_header() {
        assert_eq!(RST_ACK.header(), 0xC1);
    }

    #[test]
    fn test_crc() {
        assert_eq!(RST_ACK.crc(), 0x9B7B);
    }

    #[test]
    fn test_is_header_valid() {
        assert!(RST_ACK.is_header_valid());
    }

    #[test]
    fn test_from_buffer() {
        let buffer: Vec<u8> = vec![0xC1, 0x02, 0x02, 0x9B, 0x7B];
        assert_eq!(RstAck::try_from(buffer.as_slice()), Ok(RST_ACK));
    }
}
