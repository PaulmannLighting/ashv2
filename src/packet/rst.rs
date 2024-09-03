use std::fmt::{Display, Formatter};

use crate::error::frame::Error;
use crate::frame::Frame;

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

impl Default for Rst {
    fn default() -> Self {
        Self::new()
    }
}

impl Display for Rst {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "RST()")
    }
}

impl Frame for Rst {
    fn header(&self) -> u8 {
        self.header
    }

    fn crc(&self) -> u16 {
        self.crc
    }

    fn bytes(&self) -> impl AsRef<[u8]> {
        let [crc0, crc1] = self.crc.to_be_bytes();
        [self.header, crc0, crc1]
    }
}

impl TryFrom<&[u8]> for Rst {
    type Error = Error;

    fn try_from(buffer: &[u8]) -> Result<Self, Self::Error> {
        if buffer.len() == Self::SIZE {
            Ok(Self {
                header: buffer[0],
                crc: u16::from_be_bytes([buffer[1], buffer[2]]),
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
        assert_eq!(RST.header(), 0xC0);
    }

    #[test]
    fn test_crc() {
        assert_eq!(RST.crc(), 0x38BC);
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
