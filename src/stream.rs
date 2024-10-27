use std::io::ErrorKind;
use std::pin::Pin;
use std::task::{Context, Poll};

use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};
use tokio::sync::mpsc::error::TrySendError;
use tokio::sync::mpsc::{Receiver, Sender};

use crate::request::Request;
use crate::Payload;

/// A stream and sink for asynchronous `ASHv2` hosts.
#[derive(Debug)]
pub struct Stream {
    sender: Sender<Request>,
    receiver: Receiver<Payload>,
}

impl Stream {
    /// Create a new `AshFramed` instance.
    #[must_use]
    pub const fn new(sender: Sender<Request>, receiver: Receiver<Payload>) -> Self {
        Self { sender, receiver }
    }
}

impl AsyncWrite for Stream {
    fn poll_write(
        self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<std::io::Result<usize>> {
        // TODO: Check valid ASH frame size.
        match self.sender.try_send(Request::Data(buf.into())) {
            Ok(()) => Poll::Ready(Ok(buf.len())),
            Err(error) => Poll::Ready(Err(match error {
                TrySendError::Full(_) => ErrorKind::WouldBlock.into(),
                TrySendError::Closed(_) => ErrorKind::BrokenPipe.into(),
            })),
        }
    }

    fn poll_flush(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        Poll::Ready(Ok(()))
    }

    fn poll_shutdown(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        Poll::Ready(
            self.sender
                .try_send(Request::Shutdown)
                .map_err(|error| match error {
                    TrySendError::Full(_) => ErrorKind::WouldBlock.into(),
                    TrySendError::Closed(_) => ErrorKind::BrokenPipe.into(),
                }),
        )
    }
}

impl AsyncRead for Stream {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<std::io::Result<()>> {
        match self.receiver.poll_recv(cx) {
            Poll::Ready(Some(payload)) => {
                buf.put_slice(&payload);
                Poll::Ready(Ok(()))
            }
            Poll::Ready(None) | Poll::Pending => Poll::Pending,
        }
    }
}
