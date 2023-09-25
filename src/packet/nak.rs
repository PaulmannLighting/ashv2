use crate::Frame;
use std::fmt::{Display, Formatter};

const ACK_RDY_MASK: u8 = 0x0F;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Nak {
    header: u8,
    crc: u16,
    flag: u8,
}

impl Nak {
    /// Creates a new NAK packet.
    #[must_use]
    pub const fn new(header: u8, crc: u16, flag: u8) -> Self {
        Self { header, crc, flag }
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
        (self.header & 0xF0) == 0xA0
    }
}

#[cfg(test)]
mod tests {
    use super::Nak;
    use crate::Frame;

    const NAK1: Nak = Nak::new(0xA6, 0x34DC, 0x7E);
    const NAK2: Nak = Nak::new(0xAD, 0x85B7, 0x7E);

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
    fn test_payload() {
        assert_eq!(NAK1.payload(), None);
        assert_eq!(NAK2.payload(), None);
    }

    #[test]
    fn test_crc() {
        assert_eq!(NAK1.crc(), 0x34DC);
        assert_eq!(NAK2.crc(), 0x85B7);
    }

    #[test]
    fn test_flag() {
        assert_eq!(NAK1.flag(), 0x7E);
        assert_eq!(NAK2.flag(), 0x7E);
    }

    #[test]
    fn test_is_header_valid() {
        assert!(NAK1.is_header_valid());
        assert!(NAK2.is_header_valid());
    }
}
