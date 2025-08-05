//! Protocol definitions for the communication layer.

pub use control_byte::ControlByte;
pub use randomization::Mask;
pub use stuffing::Stuffing;

mod control_byte;
mod randomization;
mod stuffing;
