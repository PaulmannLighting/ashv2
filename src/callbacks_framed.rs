use crate::Payload;
use std::io::{Error, ErrorKind};
use std::pin::Pin;
use std::sync::mpsc::{Receiver, SyncSender, TryRecvError, TrySendError};
use std::task::{Context, Poll, Waker};
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};

/// A framed asynchronous iterator over received callback frames.
#[derive(Debug)]
pub struct CallbacksFramed {
    waker: SyncSender<Waker>,
    receiver: Receiver<Payload>,
}

impl CallbacksFramed {
    /// Create a new `CallbacksFramed` instance.
    #[must_use]
    pub const fn new(waker: SyncSender<Waker>, receiver: Receiver<Payload>) -> Self {
        Self { waker, receiver }
    }

    fn reschedule(&self, waker: Waker) -> Poll<std::io::Result<()>> {
        if let Err(error) = self.waker.try_send(waker) {
            Poll::Ready(Err(match error {
                TrySendError::Full(_) => ErrorKind::WouldBlock.into(),
                TrySendError::Disconnected(_) => ErrorKind::BrokenPipe.into(),
            }))
        } else {
            Poll::Pending
        }
    }
}

impl AsyncRead for CallbacksFramed {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<std::io::Result<()>> {
        match self.receiver.try_recv() {
            Ok(data) => {
                buf.put_slice(&data);
                Poll::Ready(Ok(()))
            }
            Err(error) => match error {
                TryRecvError::Empty => self.reschedule(cx.waker().clone()),
                TryRecvError::Disconnected => Poll::Ready(Err(Error::new(
                    ErrorKind::BrokenPipe,
                    "Receiver channel disconnected.",
                ))),
            },
        }
    }
}

impl AsyncWrite for CallbacksFramed {
    fn poll_write(
        self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
        _buf: &[u8],
    ) -> Poll<std::io::Result<usize>> {
        Poll::Ready(Err(Error::new(
            ErrorKind::Unsupported,
            "Write not supported.",
        )))
    }

    fn poll_flush(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        Poll::Ready(Err(Error::new(
            ErrorKind::Unsupported,
            "Write not supported.",
        )))
    }

    fn poll_shutdown(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        Poll::Ready(Err(Error::new(
            ErrorKind::Unsupported,
            "Write not supported.",
        )))
    }
}
