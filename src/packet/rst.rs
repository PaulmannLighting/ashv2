use std::fmt::{Display, Formatter};

pub const HEADER: u8 = 0xC0;
pub const CRC: u16 = 0x38BC;

/// Requests the NCP to perform a software reset (valid even if the NCP is in the FAILED state).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Rst;

impl Rst {
    #[must_use]
    pub const fn new() -> Self {
        Self {}
    }
}

impl Display for Rst {
    /// Display the RST packet
    ///
    /// # Examples
    /// ```
    /// use ashv2::packet::rst::Rst;
    ///
    /// let rst = Rst::new();
    /// assert_eq!(&rst.to_string(), "RST()");
    /// ```
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "RST()")
    }
}
