//! Acknowledgement (`ACK`) frame implementation.

use core::fmt::{Display, Formatter, LowerHex, UpperHex};
use std::io::{self, Error, ErrorKind};
use std::iter::{Chain, Once, once};

use crate::frame::headers;
use crate::utils::{HexSlice, WrappingU3};
use crate::validate::{CRC, Validate};

/// Acknowledgement (`ACK`) frame.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Ack {
    header: headers::Ack,
    crc: u16,
}

impl Ack {
    /// The size of the `ACK` frame in bytes.
    pub const SIZE: usize = 3;

    /// Creates a new ACK frame.
    #[must_use]
    pub const fn new(ack_num: WrappingU3, n_rdy: bool) -> Self {
        let header = headers::Ack::new(ack_num, n_rdy);

        Self {
            header,
            crc: CRC.checksum(&[header.bits()]),
        }
    }

    /// Determines whether the not-ready flag is set.
    #[must_use]
    pub const fn not_ready(&self) -> bool {
        self.header.contains(headers::Ack::NOT_READY)
    }

    /// Returns the acknowledgement number.
    #[must_use]
    pub const fn ack_num(&self) -> WrappingU3 {
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

impl Validate for Ack {
    fn crc(&self) -> u16 {
        self.crc
    }

    fn calculate_crc(&self) -> u16 {
        CRC.checksum(&[self.header.bits()])
    }
}

impl IntoIterator for Ack {
    type Item = u8;
    type IntoIter = Chain<Once<u8>, <[u8; 2] as IntoIterator>::IntoIter>;

    fn into_iter(self) -> Self::IntoIter {
        once(self.header.bits()).chain(self.crc.to_be_bytes())
    }
}

impl TryFrom<&[u8]> for Ack {
    type Error = Error;

    fn try_from(buffer: &[u8]) -> io::Result<Self> {
        let [header, crc0, crc1] = buffer else {
            return Err(if buffer.len() < Self::SIZE {
                Error::new(ErrorKind::UnexpectedEof, "Too few bytes for ACK.")
            } else {
                Error::new(ErrorKind::OutOfMemory, "Too many bytes for ACK.")
            });
        };

        Ok(Self {
            header: headers::Ack::from_bits_retain(*header),
            crc: u16::from_be_bytes([*crc0, *crc1]),
        })
    }
}

impl UpperHex for Ack {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "Ack {{ header: ")?;
        UpperHex::fmt(&self.header.bits(), f)?;
        write!(f, ", crc: ")?;
        UpperHex::fmt(&HexSlice::new(&self.crc.to_be_bytes()), f)?;
        write!(f, " }}")
    }
}

impl LowerHex for Ack {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "Ack {{ header: ")?;
        LowerHex::fmt(&self.header.bits(), f)?;
        write!(f, ", crc: ")?;
        LowerHex::fmt(&HexSlice::new(&self.crc.to_be_bytes()), f)?;
        write!(f, " }}")
    }
}

#[cfg(test)]
mod tests {
    use super::Ack;
    use crate::frame::headers;
    use crate::utils::WrappingU3;
    use crate::validate::Validate;

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
        assert_eq!(ACK1.ack_num().as_u8(), 1);
        assert_eq!(ACK2.ack_num().as_u8(), 6);
    }

    #[test]
    fn test_to_string() {
        assert_eq!(&ACK1.to_string(), "ACK(1)+");
        assert_eq!(&ACK2.to_string(), "ACK(6)-");
    }

    #[test]
    fn test_header() {
        assert_eq!(ACK1.header, headers::Ack::from_bits_retain(0x81));
        assert_eq!(ACK2.header, headers::Ack::from_bits_retain(0x8E));
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
            assert_eq!(
                Ack::new(WrappingU3::from_u8_lossy(ack_num), false)
                    .ack_num()
                    .as_u8(),
                ack_num % 8
            );
        }
    }
}
