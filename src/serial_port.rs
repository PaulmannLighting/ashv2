use num_derive::{FromPrimitive, ToPrimitive};
use num_traits::ToPrimitive;

#[cfg(windows)]
use serialport::COMPort as SerialPort;

#[cfg(unix)]
use serialport::TTYPort as SerialPort;

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, FromPrimitive, ToPrimitive)]
pub enum BaudRate {
    RstCts = 115_200,
    XOnXOff = 57_600,
}

impl From<BaudRate> for u32 {
    fn from(baud_rate: BaudRate) -> Self {
        baud_rate
            .to_u32()
            .expect("could not convert baud rate to u32")
    }
}

/// Opens a serial port depending on the local operating system.
///
/// # Errors
/// For errors please refer to [`SerialPort::open()`] and [`serialport::new()`]
pub fn open<'a>(
    path: impl Into<std::borrow::Cow<'a, str>>,
    baud_rate: BaudRate,
) -> serialport::Result<SerialPort> {
    SerialPort::open(&serialport::new(path, baud_rate.into()))
}
