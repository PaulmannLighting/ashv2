use crate::Request;
use std::io::ErrorKind;
use std::pin::Pin;
use std::sync::mpsc::{sync_channel, Receiver, SyncSender, TryRecvError, TrySendError};
use std::task::{Context, Poll, Waker};
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};

/// A framed asynchronous `ASHv2` host.
#[derive(Debug)]
pub struct AshFramed<const BUF_SIZE: usize> {
    sender: SyncSender<Request>,
    waker: SyncSender<Waker>,
    receiver: Option<Receiver<Box<[u8]>>>,
    buffer: Vec<u8>,
    result: Option<std::io::Result<Box<[u8]>>>,
}

impl<const BUF_SIZE: usize> AshFramed<BUF_SIZE> {
    /// Create a new `AshFramed` instance.
    #[must_use]
    pub const fn new(sender: SyncSender<Request>, waker: SyncSender<Waker>) -> Self {
        Self {
            sender,
            waker,
            receiver: None,
            buffer: Vec::new(),
            result: None,
        }
    }

    fn reset(&mut self) {
        self.receiver = None;
        self.buffer.clear();
        self.result = None;
    }
}

impl<const BUF_SIZE: usize> AsyncWrite for AshFramed<BUF_SIZE> {
    fn poll_write(
        mut self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<std::io::Result<usize>> {
        let len = buf.len();
        let (response_tx, response_rx) = sync_channel(BUF_SIZE);
        let request = Request::new(buf.as_ref().into(), response_tx);

        match self.sender.try_send(request) {
            Ok(()) => {
                self.as_mut().receiver.replace(response_rx);
                Poll::Ready(Ok(len))
            }
            Err(error) => match error {
                TrySendError::Full(_) => Poll::Ready(Err(ErrorKind::WouldBlock.into())),
                TrySendError::Disconnected(_) => Poll::Ready(Err(ErrorKind::BrokenPipe.into())),
            },
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

impl<const BUF_SIZE: usize> AsyncRead for AshFramed<BUF_SIZE> {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<std::io::Result<()>> {
        if let Err(error) = self.waker.try_send(cx.waker().clone()) {
            return match error {
                TrySendError::Full(_) => Poll::Ready(Err(ErrorKind::WouldBlock.into())),
                TrySendError::Disconnected(_) => Poll::Ready(Err(ErrorKind::BrokenPipe.into())),
            };
        }

        let Some(receiver) = &self.receiver else {
            return Poll::Ready(Err(ErrorKind::WouldBlock.into()));
        };

        match receiver.try_recv() {
            Ok(data) => {
                buf.put_slice(&data);
                Poll::Pending
            }
            Err(error) => match error {
                TryRecvError::Empty => Poll::Pending,
                TryRecvError::Disconnected => Poll::Ready(Ok(())),
            },
        }
    }
}
