use crate::error::frame::Error;
use crate::frame::Frame;
use crate::packet::headers;
use crate::CRC;
use std::fmt::{Display, Formatter};
use std::io::ErrorKind;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Nak {
    header: headers::Nak,
    crc: u16,
}

impl Nak {
    pub const SIZE: usize = 3;

    /// Creates a new NAK packet.
    #[must_use]
    pub const fn new(header: headers::Nak) -> Self {
        Self {
            header,
            crc: CRC.checksum(&[header.bits()]),
        }
    }

    #[must_use]
    pub fn from_ack_num(ack_num: u8) -> Self {
        Self::new(headers::Nak::new(ack_num, false, false))
    }

    /// Determines whether the not-ready flag is set.
    #[must_use]
    pub const fn not_ready(&self) -> bool {
        self.header.contains(headers::Nak::NOT_READY)
    }

    /// Return the acknowledgement number.
    #[must_use]
    pub const fn ack_num(&self) -> u8 {
        self.header.ack_num()
    }
}

impl Display for Nak {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "NAK({}){}",
            self.ack_num(),
            if self.not_ready() { '-' } else { '+' }
        )
    }
}

impl Frame for Nak {
    fn header(&self) -> u8 {
        self.header.bits()
    }

    fn crc(&self) -> u16 {
        self.crc
    }

    fn bytes(&self) -> impl AsRef<[u8]> {
        let [crc0, crc1] = self.crc.to_be_bytes();
        [self.header.bits(), crc0, crc1]
    }
}

impl TryFrom<&[u8]> for Nak {
    type Error = std::io::Error;

    fn try_from(buffer: &[u8]) -> std::io::Result<Self> {
        let [header, crc0, crc1] = buffer else {
            return Err(if buffer.len() < Self::SIZE {
                std::io::Error::new(ErrorKind::UnexpectedEof, "ASHv2 NAK: insufficient data")
            } else {
                std::io::Error::new(ErrorKind::InvalidData, "ASHv2 NAK: too much data")
            });
        };

        Ok(Self {
            header: headers::Nak::from_bits_retain(*header),
            crc: u16::from_be_bytes([*crc0, *crc1]),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::Nak;
    use crate::frame::Frame;
    use crate::packet::headers;

    const NAK1: Nak = Nak {
        header: headers::Nak::from_bits_retain(0xA6),
        crc: 0x34DC,
    };
    const NAK2: Nak = Nak {
        header: headers::Nak::from_bits_retain(0xAD),
        crc: 0x85B7,
    };

    #[test]
    fn test_ready() {
        assert!(!NAK1.not_ready());
        assert!(NAK2.not_ready());
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
    fn test_is_crc_valid() {
        assert!(NAK1.is_crc_valid());
        assert!(NAK2.is_crc_valid());
    }

    #[test]
    fn test_from_buffer() {
        let buffer1 = [0xA6, 0x34, 0xDC];
        assert_eq!(
            Nak::try_from(buffer1.as_slice()).expect("Reference frame should be a valid NAK"),
            NAK1
        );
        let buffer2 = [0xAD, 0x85, 0xB7];
        assert_eq!(
            Nak::try_from(buffer2.as_slice()).expect("Reference frame should be a valid NAK"),
            NAK2
        );
    }
}
