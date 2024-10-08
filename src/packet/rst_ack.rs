use crate::code::Code;
use crate::crc::CRC;
use crate::frame::Frame;
use crate::frame_buffer::FrameBuffer;
use crate::{HexSlice, VERSION};
use std::fmt::{Display, Formatter, LowerHex, UpperHex};
use std::io::ErrorKind;

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
    pub fn code(&self) -> Result<Code, u8> {
        Code::try_from(self.reset_code)
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

    fn buffer(&self, buffer: &mut FrameBuffer) -> Result<(), ()> {
        buffer.push(self.header).map_err(drop)?;
        buffer.push(self.version).map_err(drop)?;
        buffer.push(self.reset_code).map_err(drop)?;
        buffer.extend_from_slice(&self.crc.to_be_bytes())
    }
}

impl TryFrom<&[u8]> for RstAck {
    type Error = std::io::Error;

    fn try_from(buffer: &[u8]) -> std::io::Result<Self> {
        let [header, version, reset_code, crc0, crc1] = buffer else {
            return Err(if buffer.len() < Self::SIZE {
                std::io::Error::new(ErrorKind::UnexpectedEof, "ASHv2: Too few bytes for RSTACK.")
            } else {
                std::io::Error::new(ErrorKind::OutOfMemory, "ASHv2: Too many bytes for RSTACK.")
            });
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
    use crate::code::Code;
    use crate::frame::Frame;

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
        assert_eq!(RST_ACK.code(), Ok(Code::PowerOn));
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
    fn test_is_crc_valid() {
        assert!(RST_ACK.is_crc_valid());
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
