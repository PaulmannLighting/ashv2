mod ash_chunks;
mod command;
mod randomization;
mod response;
pub mod stuffing;

pub const FLAG: u8 = 0x7E;
pub const ESCAPE: u8 = 0x7D;
pub const X_ON: u8 = 0x11;
pub const X_OFF: u8 = 0x13;
pub const SUBSTITUTE: u8 = 0x18;
pub const CANCEL: u8 = 0x1A;
pub const WAKE: u8 = 0xFF;
pub use ash_chunks::AshChunks;
pub use command::Command;
pub use randomization::{Mask, MaskGenerator, MaskIterator};
pub use response::{Event, HandleResult, Handler, Response};
pub use stuffing::{Stuffer, Stuffing, Unstuff, Unstuffer};
