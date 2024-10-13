use crate::crc::CRC;
use crate::frame::Frame;
use crate::packet::headers;
use crate::protocol::Mask;
use crate::types::FrameBuffer;
use crate::utils::{HexSlice, WrappingU3};
use std::fmt::{Display, Formatter, LowerHex, UpperHex};
use std::io::ErrorKind;

type Payload = heapless::Vec<u8, { Data::MAX_PAYLOAD_SIZE }>;

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
    pub const MIN_PAYLOAD_SIZE: usize = 3;
    pub const MAX_PAYLOAD_SIZE: usize = 128;
    pub const BUFFER_SIZE: usize = Self::METADATA_SIZE + Self::MAX_PAYLOAD_SIZE;

    /// Creates a new data packet.
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

    /// Consumes the `Data` packet and returns its payload.
    pub fn into_payload(self) -> Payload {
        self.payload
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

impl Frame for Data {
    fn header(&self) -> u8 {
        self.header.bits()
    }

    fn crc(&self) -> u16 {
        self.crc
    }

    fn calculate_crc(&self) -> u16 {
        calculate_crc(self.header.bits(), &self.payload)
    }

    fn buffer(&self, buffer: &mut FrameBuffer) -> Result<(), ()> {
        buffer.push(self.header.bits()).map_err(drop)?;
        buffer.extend_from_slice(&self.payload)?;
        buffer.extend_from_slice(&self.crc.to_be_bytes())
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
        let mut data = self.clone();
        data.payload.mask();
        write!(f, "Data {{ header: ")?;
        LowerHex::fmt(&data.header.bits(), f)?;
        write!(f, ", unmasked_payload: ")?;
        LowerHex::fmt(&HexSlice::new(&data.payload), f)?;
        write!(f, ", crc: ")?;
        LowerHex::fmt(&HexSlice::new(&data.crc.to_be_bytes()), f)?;
        write!(f, " }}")
    }
}

impl TryFrom<&[u8]> for Data {
    type Error = std::io::Error;

    fn try_from(buffer: &[u8]) -> std::io::Result<Self> {
        let [header, payload @ .., crc0, crc1] = buffer else {
            return Err(std::io::Error::new(
                ErrorKind::UnexpectedEof,
                "Too few bytes for DATA.",
            ));
        };

        if payload.len() < Self::MIN_PAYLOAD_SIZE {
            return Err(std::io::Error::new(
                ErrorKind::UnexpectedEof,
                "Too few bytes for payload for DATA.",
            ));
        }

        Ok(Self {
            header: headers::Data::from_bits_retain(*header),
            payload: payload.try_into().map_err(|()| {
                std::io::Error::new(ErrorKind::OutOfMemory, "Payload too large for DATA.")
            })?,
            crc: u16::from_be_bytes([*crc0, *crc1]),
        })
    }
}

fn calculate_crc(header: u8, payload: &Payload) -> u16 {
    let mut bytes = heapless::Vec::<u8, { Data::MAX_PAYLOAD_SIZE + 1 }>::new();
    bytes
        .push(header)
        .expect("Buffer should have sufficient size for header.");
    bytes
        .extend_from_slice(payload)
        .expect("Buffer should have sufficient size for payload.");
    CRC.checksum(&bytes)
}

#[allow(clippy::unwrap_used)]
#[cfg(test)]
mod tests {
    use super::Data;
    use crate::crc::CRC;
    use crate::frame::Frame;
    use crate::packet::headers;
    use crate::protocol::Mask;
    use crate::types::FrameBuffer;

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
        let mut byte_representation = FrameBuffer::new();
        data.buffer(&mut byte_representation)
            .expect("Buffer should be large enough.");
        assert_eq!(&byte_representation, &[0, 67, 33, 168, 80, 155, 152]);
    }
}
