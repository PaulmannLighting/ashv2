use crate::BaudRate;

#[cfg(windows)]
pub use serialport::COMPort as SerialPortImpl;

#[cfg(unix)]
pub use serialport::TTYPort as SerialPortImpl;

/// Opens a serial port depending on the local operating system.
///
/// # Errors
/// For errors please refer to [`SerialPort::open()`] and [`serialport::new()`]
pub fn open<'a>(
    path: impl Into<std::borrow::Cow<'a, str>>,
    baud_rate: BaudRate,
) -> serialport::Result<SerialPortImpl> {
    SerialPortImpl::open(&serialport::new(path, baud_rate.into()))
}
