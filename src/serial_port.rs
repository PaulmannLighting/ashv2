use crate::BaudRate;

use serialport::{FlowControl, SerialPort};

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
    flow_control: FlowControl,
) -> serialport::Result<SerialPortImpl> {
    SerialPortImpl::open(&serialport::new(path, baud_rate.into())).and_then(|mut serial_port| {
        serial_port
            .set_flow_control(flow_control)
            .map(|_| serial_port)
    })
}
