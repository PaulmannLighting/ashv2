pub use randomization::Mask;
pub use stuffing::Stuffing;

mod randomization;
mod stuffing;

/// Flag byte to initiate the end of a frame.
pub const FLAG: u8 = 0x7E;

/// Escape byte to escape special characters.
pub const ESCAPE: u8 = 0x7D;

/// XON byte to resume transmission.
pub const X_ON: u8 = 0x11;

/// XOFF byte to pause transmission.
pub const X_OFF: u8 = 0x13;

/// Substitute byte to indicate an error.
pub const SUBSTITUTE: u8 = 0x18;

/// Cancel byte to cancel a frame.
pub const CANCEL: u8 = 0x1A;

/// Wake byte to wake up the receiver.
pub const WAKE: u8 = 0xFF;
