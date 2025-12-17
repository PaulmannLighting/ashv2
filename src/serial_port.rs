//! Miscellaneous functions for opening serial ports.

use std::borrow::Cow;

pub use serialport::{FlowControl, SerialPort, TTYPort};

use crate::BaudRate;

/// Opens a serial port depending on the local operating system.
///
/// # Errors
///
/// For errors please refer to [`SerialPortBuilder::open_native()`](serialport::SerialPortBuilder::open_native())
/// and [`serialport::new()`].
pub fn open<'a>(
    path: impl Into<Cow<'a, str>>,
    baud_rate: BaudRate,
    flow_control: FlowControl,
) -> serialport::Result<TTYPort> {
    serialport::new(path, baud_rate.into())
        .flow_control(flow_control)
        .open_native()
}
