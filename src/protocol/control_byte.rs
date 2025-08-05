/// Control bytes used in the `ASHv2` protocol.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
#[repr(u8)]
pub enum ControlByte {
    /// Flag byte to initiate the end of a frame.
    Flag = 0x7E,

    /// Escape byte to escape special characters.
    Escape = 0x7D,

    /// XON byte to resume transmission.
    Xon = 0x11,

    /// XOFF byte to pause transmission.
    Xoff = 0x13,

    /// Substitute byte to indicate an error.
    Substitute = 0x18,

    /// Cancel byte to cancel a frame.
    Cancel = 0x1A,

    /// Wake byte to wake up the receiver.
    Wake = 0xFF,
}

impl PartialEq<ControlByte> for u8 {
    fn eq(&self, other: &ControlByte) -> bool {
        *self == *other as Self
    }
}
