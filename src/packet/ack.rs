use std::fmt::{Display, Formatter};

use crate::error::frame::Error;
use crate::frame::Frame;
use crate::CRC;

const ACK_RDY_MASK: u8 = 0b0000_1000;
const ACK_NUM_MASK: u8 = 0b0000_0111;
const HEADER_PREFIX: u8 = 0x80;
const SIZE: usize = 3;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Ack {
    header: u8,
    crc: u16,
}

impl Ack {
    /// Creates a new ACK packet.
    #[must_use]
    pub const fn new(header: u8) -> Self {
        Self {
            header,
            crc: CRC.checksum(&[header]),
        }
    }

    #[must_use]
    pub const fn from_ack_num(ack_num: u8) -> Self {
        Self::new(HEADER_PREFIX + (ack_num % 8))
    }

    /// Determines whether the ready flag is set.
    #[must_use]
    pub const fn ready(&self) -> bool {
        (self.header & ACK_RDY_MASK) == 0
    }

    /// Returns the acknowledgement number.
    #[must_use]
    pub const fn ack_num(&self) -> u8 {
        self.header & ACK_NUM_MASK
    }
}

impl Display for Ack {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "ACK({}){}",
            self.ack_num(),
            if self.ready() { '+' } else { '-' }
        )
    }
}

impl Frame for Ack {
    fn header(&self) -> u8 {
        self.header
    }

    fn crc(&self) -> u16 {
        self.crc
    }

    fn is_header_valid(&self) -> bool {
        (self.header & 0xF0) == HEADER_PREFIX
    }
    fn bytes(&self) -> impl AsRef<[u8]> {
        let [crc0, crc1] = self.crc.to_be_bytes();
        [self.header, crc0, crc1]
    }
}

impl TryFrom<&[u8]> for Ack {
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
    use crate::frame::Frame;

    use super::Ack;

    const ACK1: Ack = Ack {
        header: 0x81,
        crc: 0x6059,
    };
    const ACK2: Ack = Ack {
        header: 0x8E,
        crc: 0x91B6,
    };

    #[test]
    fn test_is_valid() {
        assert!(ACK1.is_valid());
        assert!(ACK2.is_valid());
    }

    #[test]
    fn test_ready() {
        assert!(ACK1.ready());
        assert!(!ACK2.ready());
    }

    #[test]
    fn test_ack_num() {
        assert_eq!(ACK1.ack_num(), 1);
        assert_eq!(ACK2.ack_num(), 6);
    }

    #[test]
    fn test_to_string() {
        assert_eq!(&ACK1.to_string(), "ACK(1)+");
        assert_eq!(&ACK2.to_string(), "ACK(6)-");
    }

    #[test]
    fn test_header() {
        assert_eq!(ACK1.header(), 0x81);
        assert_eq!(ACK2.header(), 0x8E);
    }

    #[test]
    fn test_crc() {
        assert_eq!(ACK1.crc(), 0x6059);
        assert_eq!(ACK2.crc(), 0x91B6);
    }

    #[test]
    fn test_is_header_valid() {
        assert!(ACK1.is_header_valid());
        assert!(ACK2.is_header_valid());
    }

    #[test]
    fn test_from_buffer() {
        let buffer1: Vec<u8> = vec![0x81, 0x60, 0x59];
        assert_eq!(
            Ack::try_from(buffer1.as_slice()).expect("Reference frame should be a valid ACK"),
            ACK1
        );
        let buffer2: Vec<u8> = vec![0x8E, 0x91, 0xB6];
        assert_eq!(
            Ack::try_from(buffer2.as_slice()).expect("Reference frame should be a valid ACK"),
            ACK2
        );
    }

    #[test]
    fn from_ack_num() {
        for ack_num in u8::MIN..=u8::MAX {
            assert_eq!(Ack::from_ack_num(ack_num).ack_num(), ack_num % 8);
        }
    }
}
