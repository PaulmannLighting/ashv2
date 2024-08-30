use std::fmt::{Display, Formatter};

use log::warn;

use crate::error::frame::Error;
use crate::frame::Frame;
use crate::protocol::Mask;
use crate::CRC;

type Payload = heapless::Vec<u8, { Data::MAX_PAYLOAD_SIZE }>;
type Buffer = heapless::Vec<u8, { Data::BUFFER_SIZE }>;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Data {
    header: u8,
    payload: Payload,
    crc: u16,
}

impl Data {
    const ACK_NUM_MASK: u8 = 0b0000_0111;
    const FRAME_NUM_MASK: u8 = 0b0111_0000;
    const RETRANSMIT_MASK: u8 = 0b0000_1000;
    const FRAME_NUM_OFFSET: u8 = 4;
    const HEADER_SIZE: usize = 1;
    const CRC_CHECKSUM_SIZE: usize = 2;
    pub const METADATA_SIZE: usize = Self::HEADER_SIZE + Self::CRC_CHECKSUM_SIZE;
    pub const MIN_PAYLOAD_SIZE: usize = 3;
    pub const MAX_PAYLOAD_SIZE: usize = 128;
    pub const BUFFER_SIZE: usize = Self::METADATA_SIZE + Self::MAX_PAYLOAD_SIZE;

    /// Creates a new data packet.
    #[must_use]
    pub fn new(header: u8, payload: Payload) -> Self {
        Self {
            header,
            crc: calculate_crc(header, &payload),
            payload,
        }
    }

    #[must_use]
    pub fn create(frame_num: u8, ack_num: u8, payload: Payload) -> Self {
        Self::new(
            ((frame_num << Self::FRAME_NUM_OFFSET) & Self::FRAME_NUM_MASK)
                + (ack_num & Self::ACK_NUM_MASK),
            payload.mask().collect(),
        )
    }

    /// Returns the frame number.
    #[must_use]
    pub const fn frame_num(&self) -> u8 {
        (self.header & Self::FRAME_NUM_MASK) >> Self::FRAME_NUM_OFFSET
    }

    /// Returns the acknowledgment number.
    #[must_use]
    pub const fn ack_num(&self) -> u8 {
        self.header & Self::ACK_NUM_MASK
    }

    /// Returns the retransmit flag.
    #[must_use]
    pub const fn is_retransmission(&self) -> bool {
        (self.header & Self::RETRANSMIT_MASK) != 0
    }

    /// Returns the payload data.
    #[must_use]
    pub fn payload(&self) -> &[u8] {
        &self.payload
    }

    pub fn set_is_retransmission(&mut self, is_retransmission: bool) {
        if is_retransmission {
            self.header |= Self::RETRANSMIT_MASK;
        } else {
            self.header &= 0xFF ^ Self::RETRANSMIT_MASK;
        }

        self.crc = self.calculate_crc();
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
        self.header
    }

    fn crc(&self) -> u16 {
        self.crc
    }

    fn is_header_valid(&self) -> bool {
        true
    }

    fn calculate_crc(&self) -> u16 {
        calculate_crc(self.header, &self.payload)
    }

    fn bytes(&self) -> impl AsRef<[u8]> {
        let mut buffer = Buffer::new();
        buffer
            .push(self.header)
            .expect("Buffer should have sufficient size for header.");
        buffer
            .extend_from_slice(&self.payload)
            .expect("Buffer should have sufficient size for payload.");
        buffer
            .extend_from_slice(&self.crc.to_be_bytes())
            .expect("Buffer should have sufficient size for CRC.");
        buffer
    }
}

impl TryFrom<&[u8]> for Data {
    type Error = Error;

    fn try_from(buffer: &[u8]) -> Result<Self, Self::Error> {
        if buffer.len() < Self::METADATA_SIZE {
            return Err(Error::BufferTooSmall {
                expected: Self::METADATA_SIZE,
                found: buffer.len(),
            });
        }

        let payload = &buffer[1..(buffer.len() - 2)];

        if payload.len() < Self::MIN_PAYLOAD_SIZE {
            warn!(
                "Payload too small: {} < {}",
                payload.len(),
                Self::MIN_PAYLOAD_SIZE
            );
        }

        if payload.len() > Self::MAX_PAYLOAD_SIZE {
            warn!(
                "Payload too large: {} > {}",
                payload.len(),
                Self::MAX_PAYLOAD_SIZE
            );
        }

        Ok(Self {
            header: buffer[0],
            payload: payload.try_into().map_err(|()| Error::BufferTooSmall {
                expected: payload.len(),
                found: Self::MAX_PAYLOAD_SIZE,
            })?,
            crc: u16::from_be_bytes([buffer[buffer.len() - 2], buffer[buffer.len() - 1]]),
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
    use crate::frame::Frame;
    use crate::protocol::Mask;
    use crate::CRC;

    use super::Data;

    #[test]
    fn test_is_valid() {
        // EZSP "version" command: 00 00 00 02
        let data = Data {
            header: 0x25,
            payload: [0x00, 0x00, 0x00, 0x02].as_slice().try_into().unwrap(),
            crc: 0x1AAD,
        };
        assert!(data.is_valid());

        // EZSP "version" response: 00 80 00 02 02 11 30
        let data = Data {
            header: 0x53,
            payload: [0x00, 0x80, 0x00, 0x02, 0x02, 0x11, 0x30]
                .as_slice()
                .try_into()
                .unwrap(),
            crc: 0x6316,
        };
        assert!(data.is_valid());
    }

    #[test]
    fn test_frame_num() {
        // EZSP "version" command: 00 00 00 02
        let data = Data {
            header: 0x25,
            payload: [0x00, 0x00, 0x00, 0x02].as_slice().try_into().unwrap(),
            crc: 0x1AAD,
        };
        assert_eq!(data.frame_num(), 2);

        // EZSP "version" response: 00 80 00 02 02 11 30
        let data = Data {
            header: 0x53,
            payload: [0x00, 0x80, 0x00, 0x02, 0x02, 0x11, 0x30]
                .as_slice()
                .try_into()
                .unwrap(),
            crc: 0x6316,
        };
        assert_eq!(data.frame_num(), 5);
    }

    #[test]
    fn test_ack_num() {
        // EZSP "version" command: 00 00 00 02
        let data = Data {
            header: 0x25,
            payload: [0x00, 0x00, 0x00, 0x02].as_slice().try_into().unwrap(),
            crc: 0x1AAD,
        };
        assert_eq!(data.ack_num(), 5);

        // EZSP "version" response: 00 80 00 02 02 11 30
        let data = Data {
            header: 0x53,
            payload: [0x00, 0x80, 0x00, 0x02, 0x02, 0x11, 0x30]
                .as_slice()
                .try_into()
                .unwrap(),
            crc: 0x6316,
        };
        assert_eq!(data.ack_num(), 3);
    }

    #[test]
    fn test_retransmit() {
        // EZSP "version" command: 00 00 00 02
        let mut data = Data {
            header: 0x25,
            payload: [0x00, 0x00, 0x00, 0x02].as_slice().try_into().unwrap(),
            crc: 0x1AAD,
        };
        assert!(!data.is_retransmission());
        data.set_is_retransmission(true);
        assert!(data.is_retransmission());

        // EZSP "version" response: 00 80 00 02 02 11 30
        let mut data = Data {
            header: 0x53,
            payload: [0x00, 0x80, 0x00, 0x02, 0x02, 0x11, 0x30]
                .as_slice()
                .try_into()
                .unwrap(),
            crc: 0x6316,
        };
        assert!(!data.is_retransmission());
        data.set_is_retransmission(true);
        assert!(data.is_retransmission());
    }

    #[test]
    fn test_to_string() {
        // EZSP "version" command: 00 00 00 02
        let data = Data {
            header: 0x25,
            payload: [0x00, 0x00, 0x00, 0x02].as_slice().try_into().unwrap(),
            crc: 0x1AAD,
        };
        assert_eq!(&data.to_string(), "DATA(2, 5, 0)");

        // EZSP "version" response: 00 80 00 02 02 11 30
        let data = Data {
            header: 0x53,
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
            header: 0x25,
            payload: [0x00, 0x00, 0x00, 0x02].as_slice().try_into().unwrap(),
            crc: 0x1AAD,
        };
        assert_eq!(data.crc(), 0x1AAD);

        // EZSP "version" response: 00 80 00 02 02 11 30
        let data = Data {
            header: 0x53,
            payload: [0x00, 0x80, 0x00, 0x02, 0x02, 0x11, 0x30]
                .as_slice()
                .try_into()
                .unwrap(),
            crc: 0x6316,
        };
        assert_eq!(data.crc(), 0x6316);
    }

    #[test]
    fn test_is_header_valid() {
        // EZSP "version" command: 00 00 00 02
        let data = Data {
            header: 0x25,
            payload: [0x00, 0x00, 0x00, 0x02].as_slice().try_into().unwrap(),
            crc: 0x1AAD,
        };
        assert!(data.is_header_valid());

        // EZSP "version" response: 00 80 00 02 02 11 30
        let data = Data {
            header: 0x53,
            payload: [0x00, 0x80, 0x00, 0x02, 0x02, 0x11, 0x30]
                .as_slice()
                .try_into()
                .unwrap(),
            crc: 0x6316,
        };
        assert!(data.is_header_valid());
    }

    #[test]
    fn test_from_buffer() {
        // EZSP "version" command: 00 00 00 02
        let buffer: Vec<u8> = vec![0x25, 0x00, 0x00, 0x00, 0x02, 0x1A, 0xAD];
        let data = Data {
            header: 0x25,
            payload: [0x00, 0x00, 0x00, 0x02].as_slice().try_into().unwrap(),
            crc: 0x1AAD,
        };
        assert_eq!(Data::try_from(buffer.as_slice()).unwrap(), data);

        // EZSP "version" response: 00 80 00 02 02 11 30
        let buffer: Vec<u8> = vec![0x53, 0x00, 0x80, 0x00, 0x02, 0x02, 0x11, 0x30, 0x63, 0x16];
        let data = Data {
            header: 0x53,
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
        let masked_payload: Vec<_> = payload.into_iter().mask().collect();
        let mut crc_target = vec![header];
        crc_target.extend_from_slice(&masked_payload);
        let crc = CRC.checksum(&crc_target);
        let data = Data {
            header: 0x00,
            payload: masked_payload.as_slice().try_into().unwrap(),
            crc,
        };
        let unmasked_payload: Vec<u8> = data.payload().iter().copied().mask().collect();
        assert_eq!(unmasked_payload, payload);
        let byte_representation: Vec<_> = data.bytes().as_ref().to_vec();
        assert_eq!(byte_representation, vec![0, 67, 33, 168, 80, 155, 152]);
    }
}
