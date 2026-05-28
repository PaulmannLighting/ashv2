//! Available baud rates for the NCP (Network Co-Processor).

use serialport::FlowControl;

/// Baud rate for hardware flow control using RST/CTS.
const RST_CTS: u32 = 115_200;

/// Baud rate for software flow control using XON/XOFF.
const XON_XOFF: u32 = 57_600;

/// Available baud rates that the NCP can operate on.
pub trait BaudRate {
    /// Return the baud rate.
    fn baud_rate(&self) -> u32;
}

impl BaudRate for FlowControl {
    fn baud_rate(&self) -> u32 {
        match self {
            Self::None | Self::Software => RST_CTS,
            Self::Hardware => XON_XOFF,
        }
    }
}
