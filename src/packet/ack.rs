use crate::frame::Frame;
use crate::packet::headers;
use crate::CRC;
use std::fmt::{Display, Formatter};
use std::io::ErrorKind;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Ack {
    header: headers::Ack,
    crc: u16,
}

impl Ack {
    pub const SIZE: usize = 3;

    /// Creates a new ACK packet.
    #[must_use]
    pub const fn new(header: headers::Ack) -> Self {
        Self {
            header,
            crc: CRC.checksum(&[header.bits()]),
        }
    }

    #[must_use]
    pub fn from_ack_num(ack_num: u8) -> Self {
        Self::new(headers::Ack::new(ack_num, false, false))
    }

    /// Determines whether the not-ready flag is set.
    #[must_use]
    pub const fn not_ready(&self) -> bool {
        self.header.contains(headers::Ack::NOT_READY)
    }

    /// Returns the acknowledgement number.
    #[must_use]
    pub const fn ack_num(&self) -> u8 {
        self.header.ack_num()
    }
}

impl Display for Ack {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "ACK({}){}",
            self.ack_num(),
            if self.not_ready() { '-' } else { '+' }
        )
    }
}

impl Frame for Ack {
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

impl TryFrom<&[u8]> for Ack {
    type Error = std::io::Error;

    fn try_from(buffer: &[u8]) -> std::io::Result<Self> {
        let [header, crc0, crc1] = buffer else {
            return Err(if buffer.len() < Self::SIZE {
                std::io::Error::new(ErrorKind::UnexpectedEof, "ASHv2 ACK: insufficient data")
            } else {
                std::io::Error::new(ErrorKind::InvalidData, "ASHv2 ACK: too much data")
            });
        };

        Ok(Self {
            header: headers::Ack::from_bits_retain(*header),
            crc: u16::from_be_bytes([*crc0, *crc1]),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::Ack;
    use crate::frame::Frame;
    use crate::packet::headers;

    const ACK1: Ack = Ack {
        header: headers::Ack::from_bits_retain(0x81),
        crc: 0x6059,
    };
    const ACK2: Ack = Ack {
        header: headers::Ack::from_bits_retain(0x8E),
        crc: 0x91B6,
    };

    #[test]
    fn test_ready() {
        assert!(!ACK1.not_ready());
        assert!(ACK2.not_ready());
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
    fn test_is_crc_valid() {
        assert!(ACK1.is_crc_valid());
        assert!(ACK2.is_crc_valid());
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
