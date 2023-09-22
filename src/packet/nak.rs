use std::fmt::{Display, Formatter};

const ACK_RDY_MASK: u8 = 0x0F;

pub struct Nak {
    header: u8,
    crc: u16,
}

impl Nak {
    #[must_use]
    pub const fn new(header: u8, crc: u16) -> Self {
        Self { header, crc }
    }

    /// Determine whether the ready flag is set
    ///
    /// # Examples
    /// ````
    /// use ashv2::packet::nak::Nak;
    ///
    /// let ack = Nak::new(0xA6, 0x34DC);
    /// assert!(ack.ready());
    ///
    /// let ack = Nak::new(0xAD, 0x85B7);
    /// assert!(!ack.ready());
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
    /// let ack = Nak::new(0xA6, 0x34DC);
    /// assert_eq!(ack.ack_num(), 6);
    ///
    /// let ack = Nak::new(0xAD, 0x85B7);
    /// assert_eq!(ack.ack_num(), 5);
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
    /// let ack = Nak::new(0xA6, 0x34DC);
    /// assert_eq!(&ack.to_string(), "NAK(6)+");
    ///
    /// let ack = Nak::new(0xAD, 0x85B7);
    /// assert_eq!(&ack.to_string(), "NAK(5)-");
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
