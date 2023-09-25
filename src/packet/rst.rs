use crate::Frame;
use std::fmt::{Display, Formatter};

pub const HEADER: u8 = 0xC0;
pub const CRC: u16 = 0x38BC;

/// Requests the NCP to perform a software reset (valid even if the NCP is in the FAILED state).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Rst {
    header: u8,
    crc: u16,
    flag: u8,
}

impl Rst {
    /// Creates a new RST packet.
    #[must_use]
    pub const fn new(header: u8, crc: u16, flag: u8) -> Self {
        Self { header, crc, flag }
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
        self.header == HEADER
    }
}

#[cfg(test)]
mod tests {
    use super::Rst;
    use crate::Frame;

    const RST: Rst = Rst::new(0xC0, 0x38BC, 0x7E);

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
    fn test_payload() {
        assert_eq!(RST.payload(), None);
    }

    #[test]
    fn test_crc() {
        assert_eq!(RST.crc(), 0x38BC);
    }

    #[test]
    fn test_flag() {
        assert_eq!(RST.flag(), 0x7E);
    }

    #[test]
    fn test_is_header_valid() {
        assert!(RST.is_header_valid());
    }
}
