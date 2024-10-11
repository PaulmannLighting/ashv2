use crate::Request;
use std::io::ErrorKind;
use std::sync::mpsc::{sync_channel, Receiver, SyncSender, TryRecvError};
use std::task::Poll;
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};

/// A framed asynchronous `ASHv2` host.
#[derive(Debug)]
pub struct AshFramed<const BUF_SIZE: usize> {
    sender: SyncSender<Request>,
    receiver: Option<Receiver<Box<[u8]>>>,
}

impl<const BUF_SIZE: usize> AshFramed<BUF_SIZE> {
    /// Create a new `AshFramed` instance.
    #[must_use]
    pub const fn new(sender: SyncSender<Request>) -> Self {
        Self {
            sender,
            receiver: None,
        }
    }
}

impl<const BUF_SIZE: usize> Clone for AshFramed<BUF_SIZE> {
    fn clone(&self) -> Self {
        Self {
            sender: self.sender.clone(),
            receiver: None,
        }
    }
}

// TODO: This isn't truly async, but blocking. It should be replaced with a proper async implementation.
impl<const BUF_SIZE: usize> AsyncWrite for AshFramed<BUF_SIZE> {
    fn poll_write(
        mut self: std::pin::Pin<&mut Self>,
        _cx: &mut std::task::Context<'_>,
        buf: &[u8],
    ) -> Poll<std::io::Result<usize>> {
        let (sender, receiver) = sync_channel(BUF_SIZE);
        let request = Request::new(buf.into(), sender);
        self.receiver.replace(receiver);
        Poll::Ready(
            self.sender
                .send(request)
                .map(|()| buf.len())
                .map_err(|_| ErrorKind::BrokenPipe.into()),
        )
    }

    fn poll_flush(
        self: std::pin::Pin<&mut Self>,
        _cx: &mut std::task::Context<'_>,
    ) -> Poll<std::io::Result<()>> {
        Poll::Ready(Ok(()))
    }

    fn poll_shutdown(
        mut self: std::pin::Pin<&mut Self>,
        _cx: &mut std::task::Context<'_>,
    ) -> Poll<std::io::Result<()>> {
        self.receiver.take();
        Poll::Ready(Ok(()))
    }
}

// TODO: This isn't truly async, but blocking. It should be replaced with a proper async implementation.
impl<const BUF_SIZE: usize> AsyncRead for AshFramed<BUF_SIZE> {
    fn poll_read(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<std::io::Result<()>> {
        self.receiver.as_ref().map_or_else(
            || Poll::Ready(Ok(())),
            |receiver| {
                receiver.try_recv().map_or_else(
                    |error| match error {
                        TryRecvError::Disconnected => {
                            Poll::Ready(Err(ErrorKind::BrokenPipe.into()))
                        }
                        TryRecvError::Empty => {
                            cx.waker().wake_by_ref();
                            Poll::Pending
                        }
                    },
                    |payload| {
                        buf.put_slice(&payload);
                        Poll::Ready(Ok(()))
                    },
                )
            },
        )
    }
}
