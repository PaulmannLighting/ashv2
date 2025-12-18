//! Miscellaneous functions for opening serial ports.

use std::borrow::Cow;

use serialport::Result;
pub use serialport::{FlowControl, SerialPort};

use crate::BaudRate;

#[cfg(unix)]
type NativeSerialPort = serialport::TTYPort;

#[cfg(windows)]
type NativeSerialPort = serialport::COMPort;

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
) -> Result<NativeSerialPort> {
    serialport::new(path, baud_rate.into())
        .flow_control(flow_control)
        .open_native()
}

/// Trait for serial ports that can be cloned using their `try_clone_native` method.
pub trait TryCloneNative: Sized {
    /// Attempts to clone the `SerialPort` natively.
    ///
    /// This allows you to write and read simultaneously from the same serial connection.
    /// Please note that if you want a real asynchronous serial port you
    /// should look at [mio-serial](https://crates.io/crates/mio-serial) or
    /// [tokio-serial](https://crates.io/crates/tokio-serial).
    ///
    /// Also, you must be very careful when changing the settings of a cloned `SerialPort` : since
    /// the settings are cached on a per-object basis, trying to modify them from two different
    /// objects can cause some nasty behavior.
    ///
    /// This is the same as `SerialPort::try_clone()` but returns the concrete type instead.
    ///
    /// # Errors
    ///
    /// This function returns an error if the serial port couldn't be cloned.
    fn try_clone_native(&self) -> Result<Self>;
}

impl TryCloneNative for NativeSerialPort {
    fn try_clone_native(&self) -> Result<Self> {
        Self::try_clone_native(self)
    }
}
