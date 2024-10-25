use std::fmt::{Display, Formatter, LowerHex, UpperHex};
use std::io::ErrorKind;

use crate::crc::{Validate, CRC};
use crate::frame::headers;
use crate::to_buffer::ToBuffer;
use crate::types::FrameVec;
use crate::utils::HexSlice;
use crate::utils::WrappingU3;

/// Negative Acknowledgement (`NAK`) frame.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Nak {
    header: headers::Nak,
    crc: u16,
}

impl Nak {
    /// The size of the `NAK` frame in bytes.
    pub const SIZE: usize = 3;

    /// Creates a new NAK frame.
    #[must_use]
    pub fn new(ack_num: WrappingU3, n_rdy: bool) -> Self {
        let header = headers::Nak::new(ack_num, n_rdy);

        Self {
            header,
            crc: CRC.checksum(&[header.bits()]),
        }
    }

    /// Determines whether the not-ready flag is set.
    #[must_use]
    pub const fn not_ready(&self) -> bool {
        self.header.contains(headers::Nak::NOT_READY)
    }

    /// Return the acknowledgement number.
    #[must_use]
    pub const fn ack_num(&self) -> WrappingU3 {
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

impl Validate for Nak {
    fn crc(&self) -> u16 {
        self.crc
    }

    fn calculate_crc(&self) -> u16 {
        CRC.checksum(&[self.header.bits()])
    }
}

impl ToBuffer for Nak {
    fn buffer(&self, buffer: &mut FrameVec) -> std::io::Result<()> {
        buffer.push(self.header.bits()).map_err(|_| {
            std::io::Error::new(
                ErrorKind::OutOfMemory,
                "NAK: Could not write header to buffer",
            )
        })?;
        buffer
            .extend_from_slice(&self.crc.to_be_bytes())
            .map_err(|()| {
                std::io::Error::new(ErrorKind::OutOfMemory, "NAK: Could not write CRC to buffer")
            })
    }
}

impl TryFrom<&[u8]> for Nak {
    type Error = std::io::Error;

    fn try_from(buffer: &[u8]) -> std::io::Result<Self> {
        let [header, crc0, crc1] = buffer else {
            return Err(if buffer.len() < Self::SIZE {
                std::io::Error::new(ErrorKind::UnexpectedEof, "Too few bytes for NAK.")
            } else {
                std::io::Error::new(ErrorKind::OutOfMemory, "Too many bytes for NAK.")
            });
        };

        Ok(Self {
            header: headers::Nak::from_bits_retain(*header),
            crc: u16::from_be_bytes([*crc0, *crc1]),
        })
    }
}

impl UpperHex for Nak {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "Nak {{ header: ")?;
        UpperHex::fmt(&self.header.bits(), f)?;
        write!(f, ", crc: ")?;
        UpperHex::fmt(&HexSlice::new(&self.crc.to_be_bytes()), f)?;
        write!(f, " }}")
    }
}

impl LowerHex for Nak {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "Nak {{ header: ")?;
        LowerHex::fmt(&self.header.bits(), f)?;
        write!(f, ", crc: ")?;
        LowerHex::fmt(&HexSlice::new(&self.crc.to_be_bytes()), f)?;
        write!(f, " }}")
    }
}

#[cfg(test)]
mod tests {
    use super::Nak;
    use crate::crc::Validate;
    use crate::frame::headers;

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
        assert_eq!(NAK1.ack_num().as_u8(), 6);
        assert_eq!(NAK2.ack_num().as_u8(), 5);
    }

    #[test]
    fn test_to_string() {
        assert_eq!(&NAK1.to_string(), "NAK(6)+");
        assert_eq!(&NAK2.to_string(), "NAK(5)-");
    }

    #[test]
    fn test_header() {
        assert_eq!(NAK1.header, headers::Nak::from_bits_retain(0xA6));
        assert_eq!(NAK2.header, headers::Nak::from_bits_retain(0xAD));
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
