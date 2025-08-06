//! Data frame implementation.

use core::fmt::{Display, Formatter, LowerHex, UpperHex};
use std::io::{self, Error, ErrorKind};
use std::iter::{Chain, Copied, Once, once};

use crate::frame::headers;
use crate::protocol::Mask;
use crate::types::Payload;
use crate::utils::{HexSlice, WrappingU3};
use crate::validate::{CRC, Validate};

/// A data frame.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Data {
    header: headers::Data,
    payload: Payload,
    crc: u16,
}

impl Data {
    const HEADER_SIZE: usize = 1;
    const CRC_CHECKSUM_SIZE: usize = 2;
    const METADATA_SIZE: usize = Self::HEADER_SIZE + Self::CRC_CHECKSUM_SIZE;

    /// The minimum size of a data frame payload.
    pub const MIN_PAYLOAD_SIZE: usize = 3;

    /// The maximum size of a data frame payload.
    ///
    /// This is the tested limit on the `Siliconlabs MGM210P22A`, despite the documentation
    /// stating that `128` bytes is the limit.
    pub const MAX_PAYLOAD_SIZE: usize = 220;

    /// The size of a data frame buffer.
    pub const BUFFER_SIZE: usize = Self::METADATA_SIZE + Self::MAX_PAYLOAD_SIZE;

    /// Creates a new data frame.
    #[must_use]
    pub fn new(frame_num: WrappingU3, mut payload: Payload, ack_num: WrappingU3) -> Self {
        let header = headers::Data::new(frame_num, false, ack_num);
        payload.mask();

        Self {
            header,
            crc: calculate_crc(header.bits(), &payload),
            payload,
        }
    }

    /// Returns the frame number.
    #[must_use]
    pub const fn frame_num(&self) -> WrappingU3 {
        self.header.frame_num()
    }

    /// Returns the acknowledgment number.
    #[must_use]
    pub const fn ack_num(&self) -> WrappingU3 {
        self.header.ack_num()
    }

    /// Returns the retransmit flag.
    #[must_use]
    pub const fn is_retransmission(&self) -> bool {
        self.header.contains(headers::Data::RETRANSMIT)
    }

    /// Sets the retransmit flag.
    pub fn set_is_retransmission(&mut self, is_retransmission: bool) {
        self.header
            .set(headers::Data::RETRANSMIT, is_retransmission);
        self.crc = self.calculate_crc();
    }

    /// Consumes the `Data` frame and returns its payload.
    #[must_use]
    pub fn into_payload(self) -> Payload {
        self.payload
    }

    /// Returns a copy of the data frame with the payload unmasked.
    #[must_use]
    pub fn unmasked(&self) -> Self {
        let mut unmasked = self.clone();
        unmasked.payload.mask();
        unmasked
    }

    /// Returns an iterator over the data frame's bytes.
    pub fn iter(&self) -> impl Iterator<Item = u8> {
        self.into_iter()
    }
}

impl Display for Data {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "DATA({}, {}, {})",
            self.frame_num(),
            self.ack_num(),
            u8::from(self.is_retransmission())
        )
    }
}

impl Validate for Data {
    fn crc(&self) -> u16 {
        self.crc
    }

    fn calculate_crc(&self) -> u16 {
        calculate_crc(self.header.bits(), &self.payload)
    }
}

impl<'data> IntoIterator for &'data Data {
    type Item = u8;
    type IntoIter = Chain<
        Chain<Once<u8>, Copied<<&'data Payload as IntoIterator>::IntoIter>>,
        <[u8; 2] as IntoIterator>::IntoIter,
    >;

    fn into_iter(self) -> Self::IntoIter {
        once(self.header.bits())
            .chain(self.payload.iter().copied())
            .chain(self.crc.to_be_bytes())
    }
}

impl UpperHex for Data {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "Data {{ header: ")?;
        UpperHex::fmt(&self.header.bits(), f)?;
        write!(f, ", payload: ")?;
        UpperHex::fmt(&HexSlice::new(&self.payload), f)?;
        write!(f, ", crc: ")?;
        UpperHex::fmt(&HexSlice::new(&self.crc.to_be_bytes()), f)?;
        write!(f, " }}")
    }
}

/// Display unmasked payload for debugging.
impl LowerHex for Data {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "Data {{ header: ")?;
        LowerHex::fmt(&self.header.bits(), f)?;
        write!(f, ", payload: ")?;
        LowerHex::fmt(&HexSlice::new(&self.payload), f)?;
        write!(f, ", crc: ")?;
        LowerHex::fmt(&HexSlice::new(&self.crc.to_be_bytes()), f)?;
        write!(f, " }}")
    }
}

impl TryFrom<&[u8]> for Data {
    type Error = Error;

    fn try_from(buffer: &[u8]) -> io::Result<Self> {
        let [header, payload @ .., crc0, crc1] = buffer else {
            return Err(Error::new(
                ErrorKind::UnexpectedEof,
                "Too few bytes for DATA.",
            ));
        };

        if payload.len() < Self::MIN_PAYLOAD_SIZE {
            return Err(Error::new(
                ErrorKind::UnexpectedEof,
                "Too few bytes for payload for DATA.",
            ));
        }

        Ok(Self {
            header: headers::Data::from_bits_retain(*header),
            payload: payload.try_into().map_err(|()| {
                Error::new(
                    ErrorKind::OutOfMemory,
                    format!("Payload too large for DATA: {} bytes", payload.len()),
                )
            })?,
            crc: u16::from_be_bytes([*crc0, *crc1]),
        })
    }
}

#[inline]
fn calculate_crc(header: u8, payload: &Payload) -> u16 {
    let mut digest = CRC.digest();
    digest.update(&[header]);
    digest.update(payload);
    digest.finalize()
}

#[allow(clippy::unwrap_used)]
#[cfg(test)]
mod tests {
    use super::Data;
    use crate::frame::headers;
    use crate::protocol::Mask;
    use crate::types::RawFrame;
    use crate::validate::{CRC, Validate};

    #[test]
    fn test_frame_num() {
        // EZSP "version" command: 00 00 00 02
        let data = Data {
            header: headers::Data::from_bits_retain(0x25),
            payload: [0x00, 0x00, 0x00, 0x02].as_slice().try_into().unwrap(),
            crc: 0x1AAD,
        };
        assert_eq!(data.frame_num().as_u8(), 2);

        // EZSP "version" response: 00 80 00 02 02 11 30
        let data = Data {
            header: headers::Data::from_bits_retain(0x53),
            payload: [0x00, 0x80, 0x00, 0x02, 0x02, 0x11, 0x30]
                .as_slice()
                .try_into()
                .unwrap(),
            crc: 0x6316,
        };
        assert_eq!(data.frame_num().as_u8(), 5);
    }

    #[test]
    fn test_ack_num() {
        // EZSP "version" command: 00 00 00 02
        let data = Data {
            header: headers::Data::from_bits_retain(0x25),
            payload: [0x00, 0x00, 0x00, 0x02].as_slice().try_into().unwrap(),
            crc: 0x1AAD,
        };
        assert_eq!(data.ack_num().as_u8(), 5);

        // EZSP "version" response: 00 80 00 02 02 11 30
        let data = Data {
            header: headers::Data::from_bits_retain(0x53),
            payload: [0x00, 0x80, 0x00, 0x02, 0x02, 0x11, 0x30]
                .as_slice()
                .try_into()
                .unwrap(),
            crc: 0x6316,
        };
        assert_eq!(data.ack_num().as_u8(), 3);
    }

    #[test]
    fn test_retransmit() {
        // EZSP "version" command: 00 00 00 02
        let mut data = Data {
            header: headers::Data::from_bits_retain(0x25),
            payload: [0x00, 0x00, 0x00, 0x02].as_slice().try_into().unwrap(),
            crc: 0x1AAD,
        };
        assert!(!data.is_retransmission());
        data.set_is_retransmission(true);
        assert!(data.is_retransmission());
        assert!(data.is_crc_valid());

        // EZSP "version" response: 00 80 00 02 02 11 30
        let mut data = Data {
            header: headers::Data::from_bits_retain(0x53),
            payload: [0x00, 0x80, 0x00, 0x02, 0x02, 0x11, 0x30]
                .as_slice()
                .try_into()
                .unwrap(),
            crc: 0x6316,
        };
        assert!(!data.is_retransmission());
        data.set_is_retransmission(true);
        assert!(data.is_retransmission());
        assert!(data.is_crc_valid());
    }

    #[test]
    fn test_to_string() {
        // EZSP "version" command: 00 00 00 02
        let data = Data {
            header: headers::Data::from_bits_retain(0x25),
            payload: [0x00, 0x00, 0x00, 0x02].as_slice().try_into().unwrap(),
            crc: 0x1AAD,
        };
        assert_eq!(&data.to_string(), "DATA(2, 5, 0)");

        // EZSP "version" response: 00 80 00 02 02 11 30
        let data = Data {
            header: headers::Data::from_bits_retain(0x53),
            payload: [0x00, 0x80, 0x00, 0x02, 0x02, 0x11, 0x30]
                .as_slice()
                .try_into()
                .unwrap(),
            crc: 0x6316,
        };
        assert_eq!(&data.to_string(), "DATA(5, 3, 0)");
    }

    #[test]
    fn test_crc() {
        // EZSP "version" command: 00 00 00 02
        let data = Data {
            header: headers::Data::from_bits_retain(0x25),
            payload: [0x00, 0x00, 0x00, 0x02].as_slice().try_into().unwrap(),
            crc: 0x1AAD,
        };
        assert_eq!(data.crc(), 0x1AAD);

        // EZSP "version" response: 00 80 00 02 02 11 30
        let data = Data {
            header: headers::Data::from_bits_retain(0x53),
            payload: [0x00, 0x80, 0x00, 0x02, 0x02, 0x11, 0x30]
                .as_slice()
                .try_into()
                .unwrap(),
            crc: 0x6316,
        };
        assert_eq!(data.crc(), 0x6316);
    }

    #[test]
    fn test_is_crc_valid() {
        // EZSP "version" command: 00 00 00 02
        let data = Data {
            header: headers::Data::from_bits_retain(0x25),
            payload: [0x00, 0x00, 0x00, 0x02].as_slice().try_into().unwrap(),
            crc: 0x1AAD,
        };
        assert!(data.is_crc_valid());

        // EZSP "version" response: 00 80 00 02 02 11 30
        let data = Data {
            header: headers::Data::from_bits_retain(0x53),
            payload: [0x00, 0x80, 0x00, 0x02, 0x02, 0x11, 0x30]
                .as_slice()
                .try_into()
                .unwrap(),
            crc: 0x6316,
        };
        assert!(data.is_crc_valid());
    }

    #[test]
    fn test_from_buffer() {
        // EZSP "version" command: 00 00 00 02
        let buffer: Vec<u8> = vec![0x25, 0x00, 0x00, 0x00, 0x02, 0x1A, 0xAD];
        let data = Data {
            header: headers::Data::from_bits_retain(0x25),
            payload: [0x00, 0x00, 0x00, 0x02].as_slice().try_into().unwrap(),
            crc: 0x1AAD,
        };
        assert_eq!(Data::try_from(buffer.as_slice()).unwrap(), data);

        // EZSP "version" response: 00 80 00 02 02 11 30
        let buffer: Vec<u8> = vec![0x53, 0x00, 0x80, 0x00, 0x02, 0x02, 0x11, 0x30, 0x63, 0x16];
        let data = Data {
            header: headers::Data::from_bits_retain(0x53),
            payload: [0x00, 0x80, 0x00, 0x02, 0x02, 0x11, 0x30]
                .as_slice()
                .try_into()
                .unwrap(),
            crc: 0x6316,
        };
        assert_eq!(Data::try_from(buffer.as_slice()).unwrap(), data);
    }

    #[test]
    fn test_data_frame() {
        let header = 0x00;
        let payload = [0x01, 0x00, 0x00, 0x04];
        let mut masked_payload = payload;
        masked_payload.mask();
        let mut crc_target = vec![header];
        crc_target.extend_from_slice(&masked_payload);
        let crc = CRC.checksum(&crc_target);
        let data = Data {
            header: headers::Data::from_bits_retain(0x00),
            payload: masked_payload.as_slice().try_into().unwrap(),
            crc,
        };
        let mut unmasked_payload: Vec<u8> = data.clone().into_payload().to_vec();
        unmasked_payload.mask();
        assert_eq!(unmasked_payload, payload);
        let mut byte_representation = RawFrame::new();
        byte_representation.extend(&data);
        assert_eq!(&byte_representation, &[0, 67, 33, 168, 80, 155, 152]);
    }
}
