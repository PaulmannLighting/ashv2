use crate::Frame;
use std::fmt::{Display, Formatter};

const ACK_RDY_MASK: u8 = 0x0F;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Ack {
    header: u8,
    crc: u16,
    flag: u8,
}

impl Ack {
    /// Creates a new ACK packet.
    ///
    /// # Examples
    /// ````
    /// use ashv2::Frame;
    /// use ashv2::packet::ack::Ack;
    ///
    /// let ack = Ack::new(0x81, 0x6059, 0x7E);
    /// assert!(ack.is_valid());
    ///
    /// let ack = Ack::new(0x8E, 0x91B6, 0x7E);
    /// assert!(ack.is_valid());
    #[must_use]
    pub const fn new(header: u8, crc: u16, flag: u8) -> Self {
        Self { header, crc, flag }
    }

    /// Determine whether the ready flag is set
    ///
    /// # Examples
    /// ````
    /// use ashv2::packet::ack::Ack;
    ///
    /// let ack = Ack::new(0x81, 0x6059, 0x7E);
    /// assert!(ack.ready());
    ///
    /// let ack = Ack::new(0x8E, 0x91B6, 0x7E);
    /// assert!(!ack.ready());
    #[must_use]
    pub const fn ready(&self) -> bool {
        (self.header & ACK_RDY_MASK) <= 0x08
    }

    /// Return the acknowledgement number
    ///
    /// # Examples
    /// ```
    /// use ashv2::packet::ack::Ack;
    ///
    /// let ack = Ack::new(0x81, 0x6059, 0x7E);
    /// assert_eq!(ack.ack_num(), 1);
    ///
    /// let ack = Ack::new(0x8E, 0x91B6, 0x7E);
    /// assert_eq!(ack.ack_num(), 6);
    /// ```
    #[must_use]
    pub const fn ack_num(&self) -> u8 {
        (self.header & ACK_RDY_MASK) % 0x08
    }
}

impl Display for Ack {
    /// Display the ACK packet
    ///
    /// # Examples
    /// ```
    /// use ashv2::packet::ack::Ack;
    ///
    /// let ack = Ack::new(0x81, 0x6059, 0x7E);
    /// assert_eq!(&ack.to_string(), "ACK(1)+");
    ///
    /// let ack = Ack::new(0x8E, 0x91B6, 0x7E);
    /// assert_eq!(&ack.to_string(), "ACK(6)-");
    /// ```
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "ACK({}){}",
            self.ack_num(),
            if self.ready() { '+' } else { '-' }
        )
    }
}

impl Frame for Ack {
    /// Returns the header.
    ///
    /// # Examples
    /// ```
    /// use ashv2::Frame;
    /// use ashv2::packet::ack::Ack;
    ///
    /// let ack = Ack::new(0x81, 0x6059, 0x7E);
    /// assert_eq!(ack.header(), 0x81);
    ///
    /// let ack = Ack::new(0x8E, 0x91B6, 0x7E);
    /// assert_eq!(ack.header(), 0x8E);
    /// ```
    fn header(&self) -> u8 {
        self.header
    }

    /// Returns the payload.
    ///
    /// # Examples
    /// ```
    /// use ashv2::Frame;
    /// use ashv2::packet::ack::Ack;
    ///
    /// let ack = Ack::new(0x81, 0x6059, 0x7E);
    /// assert_eq!(ack.payload(), None);
    ///
    /// let ack = Ack::new(0x8E, 0x91B6, 0x7E);
    /// assert_eq!(ack.payload(), None);
    /// ```
    fn payload(&self) -> Option<&[u8]> {
        None
    }

    /// Returns the CRC checksum.
    ///
    /// # Examples
    /// ```
    /// use ashv2::Frame;
    /// use ashv2::packet::ack::Ack;
    ///
    /// let ack = Ack::new(0x81, 0x6059, 0x7E);
    /// assert_eq!(ack.crc(), 0x6059);
    ///
    /// let ack = Ack::new(0x8E, 0x91B6, 0x7E);
    /// assert_eq!(ack.crc(), 0x91B6);
    /// ```
    fn crc(&self) -> u16 {
        self.crc
    }

    /// Returns the flag byte.
    ///
    /// # Examples
    /// ```
    /// use ashv2::Frame;
    /// use ashv2::packet::ack::Ack;
    ///
    /// let ack = Ack::new(0x81, 0x6059, 0x7E);
    /// assert_eq!(ack.flag(), 0x7E);
    ///
    /// let ack = Ack::new(0x8E, 0x91B6, 0x7E);
    /// assert_eq!(ack.flag(), 0x7E);
    /// ```
    fn flag(&self) -> u8 {
        self.flag
    }

    /// Determines whether the header is valid.
    ///
    /// # Examples
    /// ```
    /// use ashv2::Frame;
    /// use ashv2::packet::ack::Ack;
    ///
    /// let ack = Ack::new(0x81, 0x6059, 0x7E);
    /// assert!(ack.is_header_valid());
    ///
    /// let ack = Ack::new(0x8E, 0x91B6, 0x7E);
    /// assert!(ack.is_header_valid());
    /// ```
    fn is_header_valid(&self) -> bool {
        (self.header & 0xF0) == 0x80
    }
}
