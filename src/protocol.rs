mod ash_chunks;
mod host;
mod randomization;
mod stuffing;

pub const FLAG: u8 = 0x7E;
pub const ESCAPE: u8 = 0x7D;
pub const X_ON: u8 = 0x11;
pub const X_OFF: u8 = 0x13;
pub const SUBSTITUTE: u8 = 0x18;
pub const CANCEL: u8 = 0x1A;
pub const TIMEOUT: u8 = 0xFF;
pub use ash_chunks::AshChunks;
pub use host::{Host, Request, Transaction, Worker};
pub use randomization::{Mask, MaskGenerator, MaskIterator};
pub use stuffing::{Stuffer, Stuffing, Unstuffer};
