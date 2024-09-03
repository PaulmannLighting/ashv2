use std::fmt::{Display, Formatter};

use crate::code::Code;
use crate::error::frame::Error;
use crate::frame::Frame;
use crate::{CRC, VERSION};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RstAck {
    header: u8,
    version: u8,
    reset_code: u8,
    crc: u16,
}

impl RstAck {
    pub const HEADER: u8 = 0xC1;
    pub const SIZE: usize = 5;

    #[must_use]
    pub const fn new(reset_code: u8) -> Self {
        Self {
            header: Self::HEADER,
            version: VERSION,
            reset_code,
            crc: CRC.checksum(&[Self::HEADER, VERSION, reset_code]),
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
        self.version == VERSION
    }

    /// Returns the reset code.
    #[must_use]
    pub fn code(&self) -> Option<Code> {
        Code::try_from(self.reset_code).ok()
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

    fn calculate_crc(&self) -> u16 {
        CRC.checksum(&[self.header, self.version, self.reset_code])
    }

    fn bytes(&self) -> impl AsRef<[u8]> {
        let [crc0, crc1] = self.crc.to_be_bytes();
        [self.header, self.version, self.reset_code, crc0, crc1]
    }
}

impl From<Code> for RstAck {
    fn from(code: Code) -> Self {
        Self::new(code.into())
    }
}

impl TryFrom<&[u8]> for RstAck {
    type Error = Error;

    fn try_from(buffer: &[u8]) -> Result<Self, Self::Error> {
        if buffer.len() == Self::SIZE {
            Ok(Self {
                header: buffer[0],
                version: buffer[1],
                reset_code: buffer[2],
                crc: u16::from_be_bytes([buffer[3], buffer[4]]),
            })
        } else {
            Err(Error::InvalidBufferSize {
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

    use super::RstAck;

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
    fn test_from_buffer() {
        let buffer: Vec<u8> = vec![0xC1, 0x02, 0x02, 0x9B, 0x7B];
        assert_eq!(
            RstAck::try_from(buffer.as_slice()).expect("Reference frame should be a valid RSTACK"),
            RST_ACK
        );
    }
}
