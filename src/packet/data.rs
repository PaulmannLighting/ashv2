use crate::Frame;
use std::fmt::{Display, Formatter};
use std::sync::Arc;

const ACK_NUM_MASK: u8 = 0x0F;
const FRAME_NUM_MASK: u8 = 0xF0;
const FRAME_NUM_OFFSET: u8 = 4;
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
    ///
    /// # Examples
    /// ```
    /// use ashv2::packet::data::Data;
    /// use ashv2::Frame;
    ///
    /// // EZSP "version" command: 00 00 00 02
    /// let data = Data::new(0x25, vec![0x00, 0x00, 0x00, 0x02].into(), 0x1AAD, 0x7E);
    /// assert!(data.is_valid());
    ///
    /// // EZSP "version" response: 00 80 00 02 02 11 30
    /// let data = Data::new(0x53 , vec![0x00, 0x80, 0x00, 0x02, 0x02, 0x11, 0x30].into(), 0x6316, 0x7E);
    /// assert!(data.is_valid());
    /// ```
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
    ///
    /// # Examples
    /// ```
    /// use ashv2::packet::data::Data;
    ///
    /// // EZSP "version" command: 00 00 00 02
    /// let data = Data::new(0x25, vec![0x00, 0x00, 0x00, 0x02].into(), 0x1AAD, 0x7E);
    /// assert_eq!(data.frame_num(), 2);
    ///
    /// // EZSP "version" response: 00 80 00 02 02 11 30
    /// let data = Data::new(0x53 , vec![0x00, 0x80, 0x00, 0x02, 0x02, 0x11, 0x30].into(), 0x6316, 0x7E);
    /// assert_eq!(data.frame_num(), 5);
    #[must_use]
    pub const fn frame_num(&self) -> u8 {
        (self.header & FRAME_NUM_MASK) >> FRAME_NUM_OFFSET
    }

    /// Returns the acknowledgment number.
    ///
    /// # Examples
    /// ```
    /// use ashv2::packet::data::Data;
    ///
    /// // EZSP "version" command: 00 00 00 02
    /// let data = Data::new(0x25, vec![0x00, 0x00, 0x00, 0x02].into(), 0x1AAD, 0x7E);
    /// assert_eq!(data.ack_num(), 5);
    ///
    /// // EZSP "version" response: 00 80 00 02 02 11 30
    /// let data = Data::new(0x53 , vec![0x00, 0x80, 0x00, 0x02, 0x02, 0x11, 0x30].into(), 0x6316, 0x7E);
    /// assert_eq!(data.ack_num(), 3);
    #[must_use]
    pub const fn ack_num(&self) -> u8 {
        self.header & ACK_NUM_MASK
    }

    /// Returns the retransmit flag.
    ///
    /// # Examples
    /// ```
    /// use ashv2::packet::data::Data;
    ///
    /// // EZSP "version" command: 00 00 00 02
    /// let data = Data::new(0x25, vec![0x00, 0x00, 0x00, 0x02].into(), 0x1AAD, 0x7E);
    /// assert!(!data.retransmit());
    ///
    /// // EZSP "version" response: 00 80 00 02 02 11 30
    /// let data = Data::new(0x53 , vec![0x00, 0x80, 0x00, 0x02, 0x02, 0x11, 0x30].into(), 0x6316, 0x7E);
    /// assert!(!data.retransmit());
    #[must_use]
    pub const fn retransmit(&self) -> bool {
        (self.header & RETRANSMIT_MASK) != 0
    }
}

impl Display for Data {
    /// Formats the data as a string.
    ///
    /// # Examples
    /// ```
    /// use ashv2::packet::data::Data;
    ///
    /// // EZSP "version" command: 00 00 00 02
    /// let data = Data::new(0x25, vec![0x00, 0x00, 0x00, 0x02].into(), 0x1AAD, 0x7E);
    /// assert_eq!(&data.to_string(), "DATA(2, 5, 0)");
    ///
    /// // EZSP "version" response: 00 80 00 02 02 11 30
    /// let data = Data::new(0x53 , vec![0x00, 0x80, 0x00, 0x02, 0x02, 0x11, 0x30].into(), 0x6316, 0x7E);
    /// assert_eq!(&data.to_string(), "DATA(5, 3, 0)");
    /// ```
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

    /// Returns the data payload.
    ///
    /// # Examples
    /// ```
    /// use ashv2::packet::data::Data;
    /// use ashv2::Frame;
    ///
    /// // EZSP "version" command: 00 00 00 02
    /// let data = Data::new(0x25, vec![0x00, 0x00, 0x00, 0x02].into(), 0x1AAD, 0x7E);
    /// assert_eq!(data.payload(), Some(vec![0x00, 0x00, 0x00, 0x02]));
    ///
    /// // EZSP "version" response: 00 80 00 02 02 11 30
    /// let data = Data::new(0x53 , vec![0x00, 0x80, 0x00, 0x02, 0x02, 0x11, 0x30].into(), 0x6316, 0x7E);
    /// assert_eq!(data.payload(), Some(vec![0x00, 0x80, 0x00, 0x02, 0x02, 0x11, 0x30]));
    /// ```
    fn payload(&self) -> Option<Vec<u8>> {
        Some(self.payload.to_vec())
    }

    /// Returns the CRC checksum.
    ///
    /// # Examples
    /// ```
    /// use ashv2::packet::data::Data;
    /// use ashv2::Frame;
    ///
    /// // EZSP "version" command: 00 00 00 02
    /// let data = Data::new(0x25, vec![0x00, 0x00, 0x00, 0x02].into(), 0x1AAD, 0x7E);
    /// assert_eq!(data.crc(), 0x1AAD);
    ///
    /// // EZSP "version" response: 00 80 00 02 02 11 30
    /// let data = Data::new(0x53 , vec![0x00, 0x80, 0x00, 0x02, 0x02, 0x11, 0x30].into(), 0x6316, 0x7E);
    /// assert_eq!(data.crc(), 0x6316);
    /// ```
    fn crc(&self) -> u16 {
        self.crc
    }

    /// Returns the flag byte.
    ///
    /// # Examples
    /// ```
    /// use ashv2::packet::data::Data;
    /// use ashv2::Frame;
    ///
    /// // EZSP "version" command: 00 00 00 02
    /// let data = Data::new(0x25, vec![0x00, 0x00, 0x00, 0x02].into(), 0x1AAD, 0x7E);
    /// assert_eq!(data.flag(), 0x7E);
    ///
    /// // EZSP "version" response: 00 80 00 02 02 11 30
    /// let data = Data::new(0x53 , vec![0x00, 0x80, 0x00, 0x02, 0x02, 0x11, 0x30].into(), 0x6316, 0x7E);
    /// assert_eq!(data.flag(), 0x7E);
    /// ```
    fn flag(&self) -> u8 {
        self.flag
    }

    /// Determines whether the header is valid.
    ///
    /// # Examples
    /// ```
    /// use ashv2::packet::data::Data;
    /// use ashv2::Frame;
    ///
    /// // EZSP "version" command: 00 00 00 02
    /// let data = Data::new(0x25, vec![0x00, 0x00, 0x00, 0x02].into(), 0x1AAD, 0x7E);
    /// assert!(data.is_header_valid());
    ///
    /// // EZSP "version" response: 00 80 00 02 02 11 30
    /// let data = Data::new(0x53 , vec![0x00, 0x80, 0x00, 0x02, 0x02, 0x11, 0x30].into(), 0x6316, 0x7E);
    /// assert!(data.is_header_valid());
    /// ```
    fn is_header_valid(&self) -> bool {
        true
    }
}
