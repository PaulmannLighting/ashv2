use crate::Request;
use std::sync::mpsc::{Receiver, SyncSender};
use std::task::Poll;
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};

/// A framed asynchronous `ASHv2` host.
pub struct AshFramed {
    sender: SyncSender<Request>,
    receiver: Option<Receiver<Box<[u8]>>>,
}

impl AshFramed {
    /// Create a new `AshFramed` instance.
    #[must_use]
    pub const fn new(sender: SyncSender<Request>) -> Self {
        Self {
            sender,
            receiver: None,
        }
    }
}

impl AsyncWrite for AshFramed {
    fn poll_write(
        mut self: std::pin::Pin<&mut Self>,
        _cx: &mut std::task::Context<'_>,
        buf: &[u8],
    ) -> Poll<std::io::Result<usize>> {
        let (request, receiver) = Request::new(buf.into());
        self.receiver.replace(receiver);
        Poll::Ready(
            self.sender
                .send(request)
                .map(|_| buf.len())
                .map_err(|_| std::io::ErrorKind::BrokenPipe.into()),
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

impl AsyncRead for AshFramed {
    fn poll_read(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<std::io::Result<()>> {
        self.receiver.as_ref().map_or_else(
            || Poll::Ready(Ok(())),
            |receiver| {
                receiver.try_recv().map_or_else(
                    |_| {
                        cx.waker().wake_by_ref();
                        Poll::Pending
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
