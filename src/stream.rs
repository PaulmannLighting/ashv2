use std::io::ErrorKind;
use std::pin::Pin;
use std::task::{Context, Poll};

use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};
use tokio::sync::mpsc::error::TrySendError;
use tokio::sync::mpsc::{Receiver, Sender};

use crate::Payload;

/// A stream and sink for asynchronous `ASHv2` hosts.
#[derive(Debug)]
pub struct Stream<const BUF_SIZE: usize> {
    sender: Sender<Box<[u8]>>,
    receiver: Receiver<Payload>,
    buffer: heapless::Vec<u8, BUF_SIZE>,
}

impl<const BUF_SIZE: usize> Stream<BUF_SIZE> {
    /// Create a new `AshFramed` instance.
    #[must_use]
    pub const fn new(sender: Sender<Box<[u8]>>, receiver: Receiver<Payload>) -> Self {
        Self {
            sender,
            receiver,
            buffer: heapless::Vec::new(),
        }
    }

    fn reset(&mut self) {
        self.buffer.clear();
    }
}

impl<const BUF_SIZE: usize> AsyncWrite for Stream<BUF_SIZE> {
    fn poll_write(
        self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<std::io::Result<usize>> {
        // TODO: Check valid ASH frame size.
        match self.sender.try_send(buf.into()) {
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

    fn poll_shutdown(mut self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        self.reset();
        Poll::Ready(Ok(()))
    }
}

impl<const BUF_SIZE: usize> AsyncRead for Stream<BUF_SIZE> {
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
