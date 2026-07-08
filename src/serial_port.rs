//! Miscellaneous functions for opening serial ports.

use std::borrow::Cow;

use serialport::Result;
pub use serialport::{FlowControl, SerialPort};

use crate::BaudRate;

/// The native serial port on Unix.
#[cfg(unix)]
pub type NativeSerialPort = serialport::TTYPort;

/// The native serial port on Windows.
#[cfg(windows)]
pub type NativeSerialPort = serialport::COMPort;

/// Opens a serial port depending on the local operating system.
///
/// # Errors
///
/// For errors please refer to [`SerialPortBuilder::open_native()`](serialport::SerialPortBuilder::open_native())
/// and [`serialport::new()`].
pub fn open<'a>(
    path: impl Into<Cow<'a, str>>,
    flow_control: FlowControl,
) -> Result<NativeSerialPort> {
    serialport::new(path, flow_control.baud_rate())
        .flow_control(flow_control)
        .open_native()
}
