use crate::error::frame::Error;
use crate::frame::Frame;
use std::array::IntoIter;
use std::fmt::{Display, Formatter};
use std::iter::Chain;

pub const HEADER: u8 = 0xC0;
const SIZE: usize = 3;
const CRC: u16 = 0x38BC;

/// Requests the NCP to perform a software reset (valid even if the NCP is in the FAILED state).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Rst {
    header: u8,
    crc: u16,
}

impl Rst {
    /// Creates a new RST packet.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            header: HEADER,
            crc: CRC,
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

    fn is_header_valid(&self) -> bool {
        self.header == HEADER
    }
}

#[allow(clippy::into_iter_without_iter)]
impl IntoIterator for &Rst {
    type Item = u8;
    type IntoIter = Chain<IntoIter<Self::Item, 1>, IntoIter<Self::Item, 2>>;

    fn into_iter(self) -> Self::IntoIter {
        self.header
            .to_be_bytes()
            .into_iter()
            .chain(self.crc.to_be_bytes())
    }
}

impl TryFrom<&[u8]> for Rst {
    type Error = Error;

    fn try_from(buffer: &[u8]) -> Result<Self, Self::Error> {
        if buffer.len() == SIZE {
            Ok(Self {
                header: buffer[0],
                crc: u16::from_be_bytes([buffer[1], buffer[2]]),
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
    use super::Rst;
    use crate::frame::Frame;

    const RST: Rst = Rst {
        header: 0xC0,
        crc: 0x38BC,
    };

    #[test]
    fn test_is_valid() {
        assert!(RST.is_valid());
    }

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
    fn test_is_header_valid() {
        assert!(RST.is_header_valid());
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
