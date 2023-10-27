use crate::frame::Frame;
use crate::{FrameError, CRC};
use std::array::IntoIter;
use std::fmt::{Display, Formatter};
use std::iter::Chain;

const ACK_RDY_MASK: u8 = 0x0F;
const HEADER_PREFIX: u8 = 0xA0;
const SIZE: usize = 3;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Nak {
    header: u8,
    crc: u16,
}

impl Nak {
    /// Creates a new NAK packet.
    #[must_use]
    pub const fn new(header: u8) -> Self {
        Self {
            header,
            crc: CRC.checksum(&[header]),
        }
    }

    #[must_use]
    pub const fn from_ack_num(ack_num: u8) -> Self {
        Self::new(HEADER_PREFIX + (ack_num % 0x08))
    }

    /// Determines whether the ready flag is set.
    #[must_use]
    pub const fn ready(&self) -> bool {
        (self.header & ACK_RDY_MASK) <= 0x08
    }

    /// Return the acknowledgement number.
    #[must_use]
    pub const fn ack_num(&self) -> u8 {
        self.header % 0x08
    }
}

impl Display for Nak {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "NAK({}){}",
            self.ack_num(),
            if self.ready() { '+' } else { '-' }
        )
    }
}

impl Frame for Nak {
    fn header(&self) -> u8 {
        self.header
    }

    fn crc(&self) -> u16 {
        self.crc
    }

    fn is_header_valid(&self) -> bool {
        (self.header & 0xF0) == 0xA0
    }
}

impl IntoIterator for &Nak {
    type Item = u8;
    type IntoIter = Chain<IntoIter<Self::Item, 1>, IntoIter<Self::Item, 2>>;

    fn into_iter(self) -> Self::IntoIter {
        self.header
            .to_be_bytes()
            .into_iter()
            .chain(self.crc.to_be_bytes())
    }
}

impl TryFrom<&[u8]> for Nak {
    type Error = FrameError;

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
    use super::Nak;
    use crate::frame::Frame;

    const NAK1: Nak = Nak {
        header: 0xA6,
        crc: 0x34DC,
    };
    const NAK2: Nak = Nak {
        header: 0xAD,
        crc: 0x85B7,
    };

    #[test]
    fn test_is_valid() {
        assert!(NAK1.is_valid());
        assert!(NAK2.is_valid());
    }

    #[test]
    fn test_ready() {
        assert!(NAK1.ready());
        assert!(!NAK2.ready());
    }

    #[test]
    fn test_ack_num() {
        assert_eq!(NAK1.ack_num(), 6);
        assert_eq!(NAK2.ack_num(), 5);
    }

    #[test]
    fn test_to_string() {
        assert_eq!(&NAK1.to_string(), "NAK(6)+");
        assert_eq!(&NAK2.to_string(), "NAK(5)-");
    }

    #[test]
    fn test_header() {
        assert_eq!(NAK1.header(), 0xA6);
        assert_eq!(NAK2.header(), 0xAD);
    }

    #[test]
    fn test_crc() {
        assert_eq!(NAK1.crc(), 0x34DC);
        assert_eq!(NAK2.crc(), 0x85B7);
    }

    #[test]
    fn test_is_header_valid() {
        assert!(NAK1.is_header_valid());
        assert!(NAK2.is_header_valid());
    }

    #[test]
    fn test_from_buffer() {
        let buffer1: Vec<u8> = vec![0xA6, 0x34, 0xDC];
        assert_eq!(Nak::try_from(buffer1.as_slice()).unwrap(), NAK1);
        let buffer2: Vec<u8> = vec![0xAD, 0x85, 0xB7];
        assert_eq!(Nak::try_from(buffer2.as_slice()).unwrap(), NAK2);
    }
}
