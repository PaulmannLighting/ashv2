use crate::frame::Frame;
use crate::protocol::Mask;
use crate::{FrameError, CRC};
use itertools::Itertools;
use log::warn;
use std::array::IntoIter;
use std::fmt::{Display, Formatter};
use std::iter::{Chain, Copied};
use std::ops::RangeInclusive;
use std::slice::Iter;
use std::sync::Arc;

const ACK_NUM_MASK: u8 = 0b0000_0111;
const FRAME_NUM_MASK: u8 = 0b0111_0000;
const RETRANSMIT_MASK: u8 = 0b0000_1000;
const FRAME_NUM_OFFSET: u8 = 4;
pub const MIN_PAYLOAD_SIZE: usize = 3;
const MIN_SIZE: usize = 3;
pub const MAX_PAYLOAD_SIZE: usize = 128;
const VALID_SEQS: RangeInclusive<u8> = 0..=7;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Data {
    header: u8,
    payload: Arc<[u8]>,
    crc: u16,
}

impl Data {
    /// Creates a new data packet.
    #[must_use]
    pub fn new(header: u8, payload: &[u8]) -> Self {
        Self {
            header,
            payload: payload.into(),
            crc: CRC.checksum(
                &header
                    .to_be_bytes()
                    .into_iter()
                    .chain(payload.iter().copied())
                    .collect_vec(),
            ),
        }
    }

    /// Returns the frame number.
    #[must_use]
    pub const fn frame_num(&self) -> u8 {
        (self.header & FRAME_NUM_MASK) >> FRAME_NUM_OFFSET
    }

    /// Returns the acknowledgment number.
    #[must_use]
    pub const fn ack_num(&self) -> u8 {
        self.header & ACK_NUM_MASK
    }

    /// Returns the retransmit flag.
    #[must_use]
    pub const fn is_retransmission(&self) -> bool {
        (self.header & RETRANSMIT_MASK) != 0
    }

    /// Returns the payload data.
    #[must_use]
    pub fn payload(&self) -> &[u8] {
        &self.payload
    }

    pub fn set_ack_num(&mut self, ack_num: u8) {
        self.header &= 0xFF ^ ACK_NUM_MASK;
        self.header |= ack_num & ACK_NUM_MASK;
    }

    pub fn set_is_retransmission(&mut self, is_retransmission: bool) {
        if is_retransmission {
            self.header |= RETRANSMIT_MASK;
        } else {
            self.header &= 0xFF ^ RETRANSMIT_MASK;
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
        let mut bytes = Vec::with_capacity(self.payload.len() + 1);
        bytes.push(self.header);
        bytes.extend_from_slice(&self.payload);
        CRC.checksum(&bytes)
    }
}

impl<'a> IntoIterator for &'a Data {
    type Item = u8;
    type IntoIter = Chain<
        Chain<IntoIter<Self::Item, 1>, Copied<Iter<'a, Self::Item>>>,
        IntoIter<Self::Item, 2>,
    >;

    fn into_iter(self) -> Self::IntoIter {
        self.header
            .to_be_bytes()
            .into_iter()
            .chain(self.payload.iter().copied())
            .chain(self.crc.to_be_bytes())
    }
}

impl TryFrom<&[u8]> for Data {
    type Error = FrameError;

    fn try_from(buffer: &[u8]) -> Result<Self, Self::Error> {
        if buffer.len() < MIN_SIZE {
            return Err(Self::Error::BufferTooSmall {
                expected: MIN_SIZE,
                found: buffer.len(),
            });
        }

        let payload: Arc<[u8]> = buffer[1..(buffer.len() - 2)].into();

        if payload.len() < MIN_PAYLOAD_SIZE {
            warn!("Payload too small: {} < {MIN_PAYLOAD_SIZE}", payload.len());
        }

        if payload.len() > MAX_PAYLOAD_SIZE {
            warn!("Payload too large: {} > {MAX_PAYLOAD_SIZE}", payload.len());
        }

        Ok(Self {
            header: buffer[0],
            payload,
            crc: u16::from_be_bytes([buffer[buffer.len() - 2], buffer[buffer.len() - 1]]),
        })
    }
}

impl TryFrom<(u8, &[u8])> for Data {
    type Error = FrameError;

    fn try_from((frame_num, payload): (u8, &[u8])) -> Result<Self, Self::Error> {
        if !VALID_SEQS.contains(&frame_num) {
            warn!("out of range frame number {frame_num} will be truncated");
        }

        if payload.len() < MIN_PAYLOAD_SIZE {
            Err(Self::Error::PayloadTooSmall {
                min: MIN_PAYLOAD_SIZE,
                size: payload.len(),
            })
        } else if payload.len() > MAX_PAYLOAD_SIZE {
            Err(Self::Error::PayloadTooLarge {
                max: MAX_PAYLOAD_SIZE,
                size: payload.len(),
            })
        } else {
            Ok(Self::new(
                (frame_num << FRAME_NUM_OFFSET) & FRAME_NUM_MASK,
                payload.iter().copied().mask().collect_vec().as_slice(),
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Data;
    use crate::frame::Frame;
    use crate::protocol::Mask;
    use crate::CRC;

    #[test]
    fn test_is_valid() {
        // EZSP "version" command: 00 00 00 02
        let data = Data {
            header: 0x25,
            payload: vec![0x00, 0x00, 0x00, 0x02].into(),
            crc: 0x1AAD,
        };
        assert!(data.is_valid());

        // EZSP "version" response: 00 80 00 02 02 11 30
        let data = Data {
            header: 0x53,
            payload: vec![0x00, 0x80, 0x00, 0x02, 0x02, 0x11, 0x30].into(),
            crc: 0x6316,
        };
        assert!(data.is_valid());
    }

    #[test]
    fn test_frame_num() {
        // EZSP "version" command: 00 00 00 02
        let data = Data {
            header: 0x25,
            payload: vec![0x00, 0x00, 0x00, 0x02].into(),
            crc: 0x1AAD,
        };
        assert_eq!(data.frame_num(), 2);

        // EZSP "version" response: 00 80 00 02 02 11 30
        let data = Data {
            header: 0x53,
            payload: vec![0x00, 0x80, 0x00, 0x02, 0x02, 0x11, 0x30].into(),
            crc: 0x6316,
        };
        assert_eq!(data.frame_num(), 5);
    }

    #[test]
    fn test_ack_num() {
        // EZSP "version" command: 00 00 00 02
        let data = Data {
            header: 0x25,
            payload: vec![0x00, 0x00, 0x00, 0x02].into(),
            crc: 0x1AAD,
        };
        assert_eq!(data.ack_num(), 5);

        // EZSP "version" response: 00 80 00 02 02 11 30
        let data = Data {
            header: 0x53,
            payload: vec![0x00, 0x80, 0x00, 0x02, 0x02, 0x11, 0x30].into(),
            crc: 0x6316,
        };
        assert_eq!(data.ack_num(), 3);
    }

    #[test]
    fn test_retransmit() {
        // EZSP "version" command: 00 00 00 02
        let mut data = Data {
            header: 0x25,
            payload: vec![0x00, 0x00, 0x00, 0x02].into(),
            crc: 0x1AAD,
        };
        assert!(!data.is_retransmission());
        data.set_is_retransmission(true);
        assert!(data.is_retransmission());

        // EZSP "version" response: 00 80 00 02 02 11 30
        let mut data = Data {
            header: 0x53,
            payload: vec![0x00, 0x80, 0x00, 0x02, 0x02, 0x11, 0x30].into(),
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
            payload: vec![0x00, 0x00, 0x00, 0x02].into(),
            crc: 0x1AAD,
        };
        assert_eq!(&data.to_string(), "DATA(2, 5, 0)");

        // EZSP "version" response: 00 80 00 02 02 11 30
        let data = Data {
            header: 0x53,
            payload: vec![0x00, 0x80, 0x00, 0x02, 0x02, 0x11, 0x30].into(),
            crc: 0x6316,
        };
        assert_eq!(&data.to_string(), "DATA(5, 3, 0)");
    }

    #[test]
    fn test_crc() {
        // EZSP "version" command: 00 00 00 02
        let data = Data {
            header: 0x25,
            payload: vec![0x00, 0x00, 0x00, 0x02].into(),
            crc: 0x1AAD,
        };
        assert_eq!(data.crc(), 0x1AAD);

        // EZSP "version" response: 00 80 00 02 02 11 30
        let data = Data {
            header: 0x53,
            payload: vec![0x00, 0x80, 0x00, 0x02, 0x02, 0x11, 0x30].into(),
            crc: 0x6316,
        };
        assert_eq!(data.crc(), 0x6316);
    }

    #[test]
    fn test_is_header_valid() {
        // EZSP "version" command: 00 00 00 02
        let data = Data {
            header: 0x25,
            payload: vec![0x00, 0x00, 0x00, 0x02].into(),
            crc: 0x1AAD,
        };
        assert!(data.is_header_valid());

        // EZSP "version" response: 00 80 00 02 02 11 30
        let data = Data {
            header: 0x53,
            payload: vec![0x00, 0x80, 0x00, 0x02, 0x02, 0x11, 0x30].into(),
            crc: 0x6316,
        };
        assert!(data.is_header_valid());
    }

    #[test]
    #[allow(clippy::unwrap_used)]
    fn test_from_buffer() {
        // EZSP "version" command: 00 00 00 02
        let buffer: Vec<u8> = vec![0x25, 0x00, 0x00, 0x00, 0x02, 0x1A, 0xAD];
        let data = Data {
            header: 0x25,
            payload: vec![0x00, 0x00, 0x00, 0x02].into(),
            crc: 0x1AAD,
        };
        assert_eq!(Data::try_from(buffer.as_slice()).unwrap(), data);

        // EZSP "version" response: 00 80 00 02 02 11 30
        let buffer: Vec<u8> = vec![0x53, 0x00, 0x80, 0x00, 0x02, 0x02, 0x11, 0x30, 0x63, 0x16];
        let data = Data {
            header: 0x53,
            payload: vec![0x00, 0x80, 0x00, 0x02, 0x02, 0x11, 0x30].into(),
            crc: 0x6316,
        };
        assert_eq!(Data::try_from(buffer.as_slice()).unwrap(), data);
    }

    #[test]
    fn test_data_frame() {
        let header = 0x00;
        let payload: Vec<u8> = vec![0x01, 0x00, 0x00, 0x04];
        let msaked_payload: Vec<_> = payload.clone().into_iter().mask().collect();
        let mut crc_target = vec![header];
        crc_target.extend_from_slice(&msaked_payload);
        let crc = CRC.checksum(&crc_target);
        let data = Data {
            header: 0x00,
            payload: msaked_payload.into(),
            crc,
        };
        let unmasked_payload: Vec<u8> = data.payload().iter().copied().mask().collect();
        assert_eq!(unmasked_payload, payload);
        let byte_representation: Vec<_> = (&data).into_iter().collect();
        assert_eq!(byte_representation, vec![0, 67, 33, 168, 80, 155, 152]);
    }

    #[test]
    fn test_set_ack_num() {
        let mut data = Data::new(0, &[1, 2, 3, 4]);
        let mut frame_num = data.frame_num();

        for ack_num in 0..8 {
            eprintln!("Header before: {:#b}", data.header);
            data.set_ack_num(ack_num);
            eprintln!("Header after: {:#b}", data.header);
            assert_eq!(data.ack_num(), ack_num);
            assert_eq!(data.frame_num(), frame_num);
        }

        frame_num = 1;
        data =
            Data::try_from((frame_num, [1, 2, 3, 4].as_slice())).expect("Could not create data.");

        for ack_num in 0..8 {
            eprintln!("Header before: {:#b}", data.header);
            data.set_ack_num(ack_num);
            eprintln!("Header after: {:#b}", data.header);
            assert_eq!(data.ack_num(), ack_num);
            assert_eq!(data.frame_num(), frame_num);
        }
    }
}
