//! Tokio [`AsyncRead`] adapter for serial ports used by the receiver.
//!
//! The receiver owns a cloned serial port handle and wraps it in [`AsyncSerialPort`] so the
//! inbound path can use Tokio I/O utilities. The adapter checks the underlying blocking
//! [`SerialPort`] for the number of available bytes and copies those bytes directly into
//! Tokio's [`ReadBuf`] when data is ready.

use std::io::Result;
use std::pin::Pin;
use std::task::{Context, Poll};

use serialport::SerialPort;
use tokio::io::{AsyncRead, ReadBuf};

/// Async read wrapper around a serial port.
///
/// This type is internal to the actor runtime. It does not make the wrapped serial port
/// itself non-blocking; it adapts the port's readiness query and read operation to
/// Tokio's [`AsyncRead`] polling interface.
#[derive(Debug)]
pub struct AsyncSerialPort<T>(
    /// Wrapped serial port handle.
    pub(crate) T,
);

impl<T> Unpin for AsyncSerialPort<T> {}

impl<T> AsyncRead for AsyncSerialPort<T>
where
    T: SerialPort,
{
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buffer: &mut ReadBuf<'_>,
    ) -> Poll<Result<()>> {
        if buffer.remaining() == 0 {
            return Poll::Ready(Ok(()));
        }

        let this = self.get_mut();

        match this.0.bytes_to_read() {
            Ok(bytes) => {
                if bytes == 0 {
                    cx.waker().wake_by_ref();
                    Poll::Pending
                } else {
                    let size = (bytes as usize).min(buffer.remaining());

                    if let Err(error) = this.0.read_exact(buffer.initialize_unfilled_to(size)) {
                        Poll::Ready(Err(error))
                    } else {
                        buffer.advance(size);
                        Poll::Ready(Ok(()))
                    }
                }
            }
            Err(error) => Poll::Ready(Err(error.into())),
        }
    }
}
