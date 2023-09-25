use crate::Frame;
use std::fmt::{Display, Formatter};

const ACK_RDY_MASK: u8 = 0x0F;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Ack {
    header: u8,
    crc: u16,
    flag: u8,
}

impl Ack {
    /// Creates a new ACK packet.
    #[must_use]
    pub const fn new(header: u8, crc: u16, flag: u8) -> Self {
        Self { header, crc, flag }
    }

    /// Determines whether the ready flag is set.
    #[must_use]
    pub const fn ready(&self) -> bool {
        (self.header & ACK_RDY_MASK) <= 0x08
    }

    /// Returns the acknowledgement number.
    #[must_use]
    pub const fn ack_num(&self) -> u8 {
        (self.header & ACK_RDY_MASK) % 0x08
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

    fn payload(&self) -> Option<Vec<u8>> {
        None
    }

    fn crc(&self) -> u16 {
        self.crc
    }

    fn flag(&self) -> u8 {
        self.flag
    }

    fn is_header_valid(&self) -> bool {
        (self.header & 0xF0) == 0x80
    }
}

#[cfg(test)]
mod tests {
    use super::Ack;
    use crate::Frame;

    const ACK1: Ack = Ack::new(0x81, 0x6059, 0x7E);
    const ACK2: Ack = Ack::new(0x8E, 0x91B6, 0x7E);

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
    fn test_payload() {
        assert_eq!(ACK1.payload(), None);
        assert_eq!(ACK2.payload(), None);
    }

    #[test]
    fn test_crc() {
        assert_eq!(ACK1.crc(), 0x6059);
        assert_eq!(ACK2.crc(), 0x91B6);
    }

    #[test]
    fn test_flag() {
        assert_eq!(ACK1.flag(), 0x7E);
        assert_eq!(ACK2.flag(), 0x7E);
    }

    #[test]
    fn test_is_header_valid() {
        assert!(ACK1.is_header_valid());
        assert!(ACK2.is_header_valid());
    }
}
