use std::fmt::{Display, Formatter, LowerHex, UpperHex};
use std::io::ErrorKind;

use crate::crc::CRC;
use crate::frame::Frame;
use crate::types::FrameBuffer;
use crate::HexSlice;

pub const RST: Rst = Rst::new();

/// Requests the NCP to perform a software reset (valid even if the NCP is in the FAILED state).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Rst {
    header: u8,
    crc: u16,
}

impl Rst {
    const CRC: u16 = 0x38BC;
    pub const HEADER: u8 = 0xC0;
    pub const SIZE: usize = 3;

    /// Creates a new RST packet.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            header: Self::HEADER,
            crc: Self::CRC,
        }
    }
}

impl Display for Rst {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "RST()")
    }
}

impl Frame for Rst {
    fn crc(&self) -> u16 {
        self.crc
    }

    fn calculate_crc(&self) -> u16 {
        CRC.checksum(&[self.header])
    }

    fn buffer(&self, buffer: &mut FrameBuffer) -> Result<(), ()> {
        buffer.push(self.header).map_err(drop)?;
        buffer.extend_from_slice(&self.crc.to_be_bytes())
    }
}

impl TryFrom<&[u8]> for Rst {
    type Error = std::io::Error;

    fn try_from(buffer: &[u8]) -> std::io::Result<Self> {
        let [header, crc0, crc1] = buffer else {
            return Err(if buffer.len() < Self::SIZE {
                std::io::Error::new(ErrorKind::UnexpectedEof, "Too few bytes for RST.")
            } else {
                std::io::Error::new(ErrorKind::OutOfMemory, "Too many bytes for RST.")
            });
        };

        Ok(Self {
            header: *header,
            crc: u16::from_be_bytes([*crc0, *crc1]),
        })
    }
}

impl UpperHex for Rst {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "Rst {{ header: ")?;
        UpperHex::fmt(&self.header, f)?;
        write!(f, ", crc: ")?;
        UpperHex::fmt(&HexSlice::new(&self.crc.to_be_bytes()), f)?;
        write!(f, " }}")
    }
}

impl LowerHex for Rst {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "Rst {{ header: ")?;
        LowerHex::fmt(&self.header, f)?;
        write!(f, ", crc: ")?;
        LowerHex::fmt(&HexSlice::new(&self.crc.to_be_bytes()), f)?;
        write!(f, " }}")
    }
}

#[cfg(test)]
mod tests {
    use crate::frame::Frame;

    use super::Rst;

    const RST: Rst = Rst {
        header: 0xC0,
        crc: 0x38BC,
    };

    #[test]
    fn test_to_string() {
        assert_eq!(&RST.to_string(), "RST()");
    }

    #[test]
    fn test_header() {
        assert_eq!(RST.header, 0xC0);
    }

    #[test]
    fn test_crc() {
        assert_eq!(RST.crc(), 0x38BC);
    }

    #[test]
    fn test_is_crc_valid() {
        assert!(RST.is_crc_valid());
    }

    #[test]
    fn test_from_buffer() {
        let buffer: Vec<u8> = vec![0xC0, 0x38, 0xBC];
        assert_eq!(
            Rst::try_from(buffer.as_slice()).expect("Reference frame should be a valid RST."),
            RST
        );
    }
}
