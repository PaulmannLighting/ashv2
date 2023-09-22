use crate::Frame;
use std::fmt::{Display, Formatter};

const ACK_RDY_MASK: u8 = 0x0F;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Nak {
    header: u8,
    crc: u16,
    flag: u8,
}

impl Nak {
    /// Creates a new NAK packet.
    ///
    /// # Examples
    /// ```
    /// use ashv2::Frame;
    /// use ashv2::packet::nak::Nak;
    ///
    /// let nak = Nak::new(0xA6, 0x34DC, 0x7E);
    /// assert!(nak.is_valid());
    ///
    /// let nak = Nak::new(0xAD, 0x85B7, 0x7E);
    /// assert!(nak.is_valid());
    /// ```
    #[must_use]
    pub const fn new(header: u8, crc: u16, flag: u8) -> Self {
        Self { header, crc, flag }
    }

    /// Determine whether the ready flag is set
    ///
    /// # Examples
    /// ````
    /// use ashv2::packet::nak::Nak;
    ///
    /// let nak = Nak::new(0xA6, 0x34DC, 0x7E);
    /// assert!(nak.ready());
    ///
    /// let nak = Nak::new(0xAD, 0x85B7, 0x7E);
    /// assert!(!nak.ready());
    #[must_use]
    pub const fn ready(&self) -> bool {
        (self.header & ACK_RDY_MASK) <= 0x08
    }

    /// Return the acknowledgement number
    ///
    /// # Examples
    /// ````
    /// use ashv2::packet::nak::Nak;
    ///
    /// let nak = Nak::new(0xA6, 0x34DC, 0x7E);
    /// assert_eq!(nak.ack_num(), 6);
    ///
    /// let nak = Nak::new(0xAD, 0x85B7, 0x7E);
    /// assert_eq!(nak.ack_num(), 5);
    #[must_use]
    pub const fn ack_num(&self) -> u8 {
        (self.header & ACK_RDY_MASK) % 0x08
    }
}

impl Display for Nak {
    /// Display the NAK packet
    ///
    /// # Examples
    /// ```
    /// use ashv2::packet::nak::Nak;
    ///
    /// let nak = Nak::new(0xA6, 0x34DC, 0x7E);
    /// assert_eq!(&nak.to_string(), "NAK(6)+");
    ///
    /// let nak = Nak::new(0xAD, 0x85B7, 0x7E);
    /// assert_eq!(&nak.to_string(), "NAK(5)-");
    /// ```
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "NAK({}){}",
            self.ack_num(),
            if self.ready() { '+' } else { '-' }
        )
    }
}

impl Frame for Nak {
    /// Returns the header.
    ///
    /// # Examples
    /// ```
    /// use ashv2::Frame;
    /// use ashv2::packet::nak::Nak;
    ///
    /// let nak = Nak::new(0xA6, 0x34DC, 0x7E);
    /// assert_eq!(nak.header(), 0xA6);
    ///
    /// let nak = Nak::new(0xAD, 0x85B7, 0x7E);
    /// assert_eq!(nak.header(), 0xAD);
    /// ```
    fn header(&self) -> u8 {
        self.header
    }

    /// Returns the payload.
    ///
    /// # Examples
    /// ```
    /// use ashv2::Frame;
    /// use ashv2::packet::nak::Nak;
    ///
    /// let nak = Nak::new(0xA6, 0x34DC, 0x7E);
    /// assert_eq!(nak.payload(), None);
    ///
    /// let nak = Nak::new(0xAD, 0x85B7, 0x7E);
    /// assert_eq!(nak.payload(), None);
    /// ```
    fn payload(&self) -> Option<Vec<u8>> {
        None
    }

    /// Returns the CRC checksum.
    ///
    /// # Examples
    /// ```
    /// use ashv2::Frame;
    /// use ashv2::packet::nak::Nak;
    ///
    /// let nak = Nak::new(0xA6, 0x34DC, 0x7E);
    /// assert_eq!(nak.crc(), 0x34DC);
    ///
    /// let nak = Nak::new(0xAD, 0x85B7, 0x7E);
    /// assert_eq!(nak.crc(), 0x85B7);
    /// ```
    fn crc(&self) -> u16 {
        self.crc
    }

    /// Returns the flag byte.
    ///
    /// # Examples
    /// ```
    /// use ashv2::Frame;
    /// use ashv2::packet::nak::Nak;
    ///
    /// let nak = Nak::new(0xA6, 0x34DC, 0x7E);
    /// assert_eq!(nak.flag(), 0x7E);
    ///
    /// let nak = Nak::new(0xAD, 0x85B7, 0x7E);
    /// assert_eq!(nak.flag(), 0x7E);
    /// ```
    fn flag(&self) -> u8 {
        self.flag
    }

    /// Determines whether the header is valid.
    ///
    /// # Examples
    /// ```
    /// use ashv2::Frame;
    /// use ashv2::packet::nak::Nak;
    ///
    /// let nak = Nak::new(0xA6, 0x34DC, 0x7E);
    /// assert!(nak.is_header_valid());
    ///
    /// let nak = Nak::new(0xAD, 0x85B7, 0x7E);
    /// assert!(nak.is_header_valid());
    /// ```
    fn is_header_valid(&self) -> bool {
        (self.header & 0xF0) == 0xA0
    }
}
