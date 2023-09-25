use crate::Frame;
use std::fmt::{Display, Formatter};
use std::sync::Arc;

const ACK_NUM_MASK: u8 = 0x0F;
const FRAME_NUM_MASK: u8 = 0xF0;
const FRAME_NUM_OFFSET: u8 = 4;
const MIN_SIZE: usize = 4;
const RETRANSMIT_MASK: u8 = 0x08;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Data {
    header: u8,
    payload: Arc<[u8]>,
    crc: u16,
    flag: u8,
}

impl Data {
    /// Creates a new data packet.
    #[must_use]
    pub const fn new(header: u8, payload: Arc<[u8]>, crc: u16, flag: u8) -> Self {
        Self {
            header,
            payload,
            crc,
            flag,
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
    pub const fn retransmit(&self) -> bool {
        (self.header & RETRANSMIT_MASK) != 0
    }
}

impl Display for Data {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "DATA({}, {}, {})",
            self.frame_num(),
            self.ack_num(),
            u8::from(self.retransmit())
        )
    }
}

impl Frame for Data {
    fn header(&self) -> u8 {
        self.header
    }

    fn payload(&self) -> Option<Vec<u8>> {
        Some(self.payload.to_vec())
    }

    fn crc(&self) -> u16 {
        self.crc
    }

    fn flag(&self) -> u8 {
        self.flag
    }

    fn is_header_valid(&self) -> bool {
        true
    }
}

impl TryFrom<&[u8]> for Data {
    type Error = crate::Error;

    fn try_from(buffer: &[u8]) -> Result<Self, Self::Error> {
        if buffer.len() >= MIN_SIZE {
            Ok(Self::new(
                buffer[0],
                buffer[1..(buffer.len() - 3)].into(),
                u16::from_be_bytes([buffer[buffer.len() - 3], buffer[buffer.len() - 2]]),
                buffer[buffer.len() - 1],
            ))
        } else {
            Err(Self::Error::BufferTooSmall(MIN_SIZE))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Data;
    use crate::Frame;

    #[test]
    fn test_is_valid() {
        // EZSP "version" command: 00 00 00 02
        let data = Data::new(0x25, vec![0x00, 0x00, 0x00, 0x02].into(), 0x1AAD, 0x7E);
        assert!(data.is_valid());

        // EZSP "version" response: 00 80 00 02 02 11 30
        let data = Data::new(
            0x53,
            vec![0x00, 0x80, 0x00, 0x02, 0x02, 0x11, 0x30].into(),
            0x6316,
            0x7E,
        );
        assert!(data.is_valid());
    }

    #[test]
    fn test_frame_num() {
        // EZSP "version" command: 00 00 00 02
        let data = Data::new(0x25, vec![0x00, 0x00, 0x00, 0x02].into(), 0x1AAD, 0x7E);
        assert_eq!(data.frame_num(), 2);

        // EZSP "version" response: 00 80 00 02 02 11 30
        let data = Data::new(
            0x53,
            vec![0x00, 0x80, 0x00, 0x02, 0x02, 0x11, 0x30].into(),
            0x6316,
            0x7E,
        );
        assert_eq!(data.frame_num(), 5);
    }

    #[test]
    fn test_ack_num() {
        // EZSP "version" command: 00 00 00 02
        let data = Data::new(0x25, vec![0x00, 0x00, 0x00, 0x02].into(), 0x1AAD, 0x7E);
        assert_eq!(data.ack_num(), 5);

        // EZSP "version" response: 00 80 00 02 02 11 30
        let data = Data::new(
            0x53,
            vec![0x00, 0x80, 0x00, 0x02, 0x02, 0x11, 0x30].into(),
            0x6316,
            0x7E,
        );
        assert_eq!(data.ack_num(), 3);
    }

    #[test]
    fn test_retransmit() {
        // EZSP "version" command: 00 00 00 02
        let data = Data::new(0x25, vec![0x00, 0x00, 0x00, 0x02].into(), 0x1AAD, 0x7E);
        assert!(!data.retransmit());

        // EZSP "version" response: 00 80 00 02 02 11 30
        let data = Data::new(
            0x53,
            vec![0x00, 0x80, 0x00, 0x02, 0x02, 0x11, 0x30].into(),
            0x6316,
            0x7E,
        );
        assert!(!data.retransmit());
    }

    #[test]
    fn test_to_string() {
        // EZSP "version" command: 00 00 00 02
        let data = Data::new(0x25, vec![0x00, 0x00, 0x00, 0x02].into(), 0x1AAD, 0x7E);
        assert_eq!(&data.to_string(), "DATA(2, 5, 0)");

        // EZSP "version" response: 00 80 00 02 02 11 30
        let data = Data::new(
            0x53,
            vec![0x00, 0x80, 0x00, 0x02, 0x02, 0x11, 0x30].into(),
            0x6316,
            0x7E,
        );
        assert_eq!(&data.to_string(), "DATA(5, 3, 0)");
    }

    #[test]
    fn test_payload() {
        // EZSP "version" command: 00 00 00 02
        let data = Data::new(0x25, vec![0x00, 0x00, 0x00, 0x02].into(), 0x1AAD, 0x7E);
        assert_eq!(data.payload(), Some(vec![0x00, 0x00, 0x00, 0x02]));

        // EZSP "version" response: 00 80 00 02 02 11 30
        let data = Data::new(
            0x53,
            vec![0x00, 0x80, 0x00, 0x02, 0x02, 0x11, 0x30].into(),
            0x6316,
            0x7E,
        );
        assert_eq!(
            data.payload(),
            Some(vec![0x00, 0x80, 0x00, 0x02, 0x02, 0x11, 0x30])
        );
    }

    #[test]
    fn test_crc() {
        // EZSP "version" command: 00 00 00 02
        let data = Data::new(0x25, vec![0x00, 0x00, 0x00, 0x02].into(), 0x1AAD, 0x7E);
        assert_eq!(data.crc(), 0x1AAD);

        // EZSP "version" response: 00 80 00 02 02 11 30
        let data = Data::new(
            0x53,
            vec![0x00, 0x80, 0x00, 0x02, 0x02, 0x11, 0x30].into(),
            0x6316,
            0x7E,
        );
        assert_eq!(data.crc(), 0x6316);
    }

    #[test]
    fn test_flag() {
        // EZSP "version" command: 00 00 00 02
        let data = Data::new(0x25, vec![0x00, 0x00, 0x00, 0x02].into(), 0x1AAD, 0x7E);
        assert_eq!(data.flag(), 0x7E);

        // EZSP "version" response: 00 80 00 02 02 11 30
        let data = Data::new(
            0x53,
            vec![0x00, 0x80, 0x00, 0x02, 0x02, 0x11, 0x30].into(),
            0x6316,
            0x7E,
        );
        assert_eq!(data.flag(), 0x7E);
    }

    #[test]
    fn test_is_header_valid() {
        // EZSP "version" command: 00 00 00 02
        let data = Data::new(0x25, vec![0x00, 0x00, 0x00, 0x02].into(), 0x1AAD, 0x7E);
        assert!(data.is_header_valid());

        // EZSP "version" response: 00 80 00 02 02 11 30
        let data = Data::new(
            0x53,
            vec![0x00, 0x80, 0x00, 0x02, 0x02, 0x11, 0x30].into(),
            0x6316,
            0x7E,
        );
        assert!(data.is_header_valid());
    }

    #[test]
    fn test_from_buffer() {
        // EZSP "version" command: 00 00 00 02
        let buffer: Vec<u8> = vec![0x25, 0x00, 0x00, 0x00, 0x02, 0x1A, 0xAD, 0x7E];
        let data = Data::new(0x25, vec![0x00, 0x00, 0x00, 0x02].into(), 0x1AAD, 0x7E);
        assert_eq!(Data::try_from(buffer.as_slice()), Ok(data));

        // EZSP "version" response: 00 80 00 02 02 11 30
        let buffer: Vec<u8> = vec![
            0x53, 0x00, 0x80, 0x00, 0x02, 0x02, 0x11, 0x30, 0x63, 0x16, 0x7E,
        ];
        let data = Data::new(
            0x53,
            vec![0x00, 0x80, 0x00, 0x02, 0x02, 0x11, 0x30].into(),
            0x6316,
            0x7E,
        );
        assert!(data.is_valid());
        assert_eq!(Data::try_from(buffer.as_slice()), Ok(data));
    }
}
