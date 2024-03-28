use crate::code::Code;
use crate::error::frame::Error;
use crate::frame::Frame;
use crate::CRC;
use num_traits::FromPrimitive;
use std::fmt::{Display, Formatter};

pub const HEADER: u8 = 0xC1;
const SIZE: usize = 5;
const VERSION: u8 = 0x02;

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
    pub fn new(header: u8, reset_code: Code) -> Self {
        Self {
            header,
            version: VERSION,
            reset_code: reset_code.clone().into(),
            crc: CRC.checksum(&[header, VERSION, reset_code.into()]),
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
        write!(f, "RSTACK({:#04X}, {:#04X})", self.version, self.reset_code)
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

impl TryFrom<&[u8]> for RstAck {
    type Error = Error;

    fn try_from(buffer: &[u8]) -> Result<Self, Self::Error> {
        if buffer.len() == SIZE {
            Ok(Self {
                header: buffer[0],
                version: buffer[1],
                reset_code: buffer[2],
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
    use crate::frame::Frame;
    use crate::Code;

    use super::RstAck;

    const RST_ACK: RstAck = RstAck {
        header: 0xC1,
        version: 0x02,
        reset_code: 0x02,
        crc: 0x9B7B,
    };

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
        assert_eq!(
            RstAck::try_from(buffer.as_slice()).expect("Reference frame should be a valid RSTACK"),
            RST_ACK
        );
    }
}
