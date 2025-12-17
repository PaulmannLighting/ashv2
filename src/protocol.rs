//! Protocol definitions for the communication layer.

pub use self::control_byte::ControlByte;
pub use self::randomization::Mask;
pub use self::stuffing::{Stuff, Unstuff};

mod control_byte;
mod randomization;
mod stuffing;
