use crate::{Frame, CRC};
use std::fmt::{Display, Formatter};

const ACK_RDY_MASK: u8 = 0x0F;
const SIZE: usize = 3;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Nak {
    header: u8,
    crc: u16,
}

impl Nak {
    /// Creates a new NAK packet.
    #[must_use]
    pub const fn new(header: u8, crc: u16) -> Self {
        Self { header, crc }
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

impl From<&Nak> for Vec<u8> {
    fn from(nak: &Nak) -> Self {
        let mut bytes = Vec::with_capacity(SIZE);
        bytes.push(nak.header);
        bytes.extend_from_slice(&nak.crc.to_be_bytes());
        bytes
    }
}

impl From<u8> for Nak {
    fn from(ack_num: u8) -> Self {
        let header = 0xA0 + (ack_num % 0x08);
        Self::new(header, CRC.checksum(&[header]))
    }
}

impl TryFrom<&[u8]> for Nak {
    type Error = crate::Error;

    fn try_from(buffer: &[u8]) -> Result<Self, Self::Error> {
        if buffer.len() == SIZE {
            Ok(Self::new(
                buffer[0],
                u16::from_be_bytes([buffer[1], buffer[2]]),
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
    use super::Nak;
    use crate::Frame;

    const NAK1: Nak = Nak::new(0xA6, 0x34DC);
    const NAK2: Nak = Nak::new(0xAD, 0x85B7);

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
        assert_eq!(Nak::try_from(buffer1.as_slice()), Ok(NAK1));
        let buffer2: Vec<u8> = vec![0xAD, 0x85, 0xB7];
        assert_eq!(Nak::try_from(buffer2.as_slice()), Ok(NAK2));
    }
}
