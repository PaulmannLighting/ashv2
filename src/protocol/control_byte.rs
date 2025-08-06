use num_derive::FromPrimitive;
use num_traits::FromPrimitive;

/// Escape byte to escape special characters.
pub const ESCAPE: u8 = 0x7D;

/// Control bytes used in the `ASHv2` protocol.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, FromPrimitive)]
#[repr(u8)]
pub enum ControlByte {
    /// Flag byte to initiate the end of a frame.
    Flag = 0x7E,

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

impl From<ControlByte> for u8 {
    fn from(byte: ControlByte) -> Self {
        byte as Self
    }
}

impl TryFrom<u8> for ControlByte {
    type Error = u8;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        Self::from_u8(value).ok_or(value)
    }
}
