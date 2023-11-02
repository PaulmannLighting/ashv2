use crate::BaudRate;

use serialport::FlowControl;

#[cfg(windows)]
use serialport::COMPort as SerialPort;

#[cfg(unix)]
use serialport::TTYPort as SerialPort;

/// Opens a serial port depending on the local operating system.
///
/// # Errors
/// For errors please refer to [`SerialPort::open()`] and [`serialport::new()`]
pub fn open<'a>(
    path: impl Into<std::borrow::Cow<'a, str>>,
    baud_rate: BaudRate,
    flow_control: FlowControl,
) -> serialport::Result<SerialPort> {
    SerialPort::open(&serialport::new(path, baud_rate.into()).flow_control(flow_control))
}
