use std::fmt::{Display, Formatter};

const ACK_RDY_MASK: u8 = 0x0F;

pub struct Ack {
    header: u8,
    crc: u16,
}

impl Ack {
    #[must_use]
    pub const fn new(header: u8, crc: u16) -> Self {
        Self { header, crc }
    }

    /// Determine whether the ready flag is set
    ///
    /// # Examples
    /// ````
    /// use ashv2::packet::ack::Ack;
    ///
    /// let ack = Ack::new(0x81, 0x6059);
    /// assert!(ack.ready());
    ///
    /// let ack = Ack::new(0x8E, 0x91B6);
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
    /// let ack = Ack::new(0x81, 0x6059);
    /// assert_eq!(ack.ack_num(), 1);
    ///
    /// let ack = Ack::new(0x8E, 0x91B6);
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
    /// let ack = Ack::new(0x81, 0x6059);
    /// assert_eq!(&ack.to_string(), "ACK(1)+");
    ///
    /// let ack = Ack::new(0x8E, 0x91B6);
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
