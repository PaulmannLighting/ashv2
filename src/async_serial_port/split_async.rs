use serialport::SerialPort;

use super::{AsyncSerialPort, Reader, Writer};

/// Extension trait for splitting a blocking serial port into asynchronous reader and writer handles.
pub trait SplitAsync {
    /// Spawns an asynchronous serial port worker and returns its reader and writer handles.
    fn split_async(self, channel_size: usize) -> (Reader, Writer);
}

impl<T> SplitAsync for T
where
    T: SerialPort + Send + 'static,
{
    fn split_async(self, channel_size: usize) -> (Reader, Writer) {
        AsyncSerialPort::new(self).spawn(channel_size)
    }
}
