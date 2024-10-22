use std::io::{Error, ErrorKind};
use std::pin::Pin;
use std::sync::mpsc::{sync_channel, Receiver, SyncSender, TryRecvError, TrySendError};
use std::task::{Context, Poll, Waker};

use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};

use crate::protocol::Stuffing;
use crate::types::FrameBuffer;
use crate::{Payload, Request, FLAG};

/// A framed asynchronous `ASHv2` host.
#[derive(Debug)]
pub struct AshFramed<const BUF_SIZE: usize> {
    sender: SyncSender<Request>,
    waker: SyncSender<Waker>,
    channel_size: usize,
    receiver: Option<Receiver<Payload>>,
    buffer: heapless::Vec<u8, BUF_SIZE>,
    frame: FrameBuffer,
}

impl<const BUF_SIZE: usize> AshFramed<BUF_SIZE> {
    /// Create a new `AshFramed` instance.
    #[must_use]
    pub const fn new(
        sender: SyncSender<Request>,
        waker: SyncSender<Waker>,
        channel_size: usize,
    ) -> Self {
        Self {
            sender,
            waker,
            channel_size,
            receiver: None,
            buffer: heapless::Vec::new(),
            frame: FrameBuffer::new(),
        }
    }

    /// Buffer a frame.
    ///
    /// Stuff the frame and append the flag byte.
    fn buffer_frame(&mut self, data: &[u8]) -> Result<(), Error> {
        self.frame.clear();
        self.frame
            .extend_from_slice(data)
            .map_err(|()| Error::new(ErrorKind::OutOfMemory, "Frame buffer overflow."))?;
        self.frame.stuff()?;
        self.frame
            .push(FLAG)
            .map_err(|_| Error::new(ErrorKind::OutOfMemory, "Frame buffer overflow."))?;
        self.buffer
            .extend_from_slice(&self.frame)
            .map_err(|()| Error::new(ErrorKind::OutOfMemory, "Buffer full."))?;
        Ok(())
    }

    fn reset(&mut self) {
        self.receiver = None;
        self.buffer.clear();
    }

    fn reschedule(&mut self, waker: Waker) -> Poll<std::io::Result<()>> {
        if let Err(error) = self.waker.try_send(waker) {
            self.buffer.clear();
            Poll::Ready(Err(try_send_error_to_io_error(&error)))
        } else {
            Poll::Pending
        }
    }
}

impl<const BUF_SIZE: usize> AsyncWrite for AshFramed<BUF_SIZE> {
    fn poll_write(
        mut self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<std::io::Result<usize>> {
        let len = buf.len();
        let (response_tx, response_rx) = sync_channel(self.channel_size);
        let request = Request::new(buf.as_ref().into(), response_tx);

        match self.sender.try_send(request) {
            Ok(()) => {
                self.receiver.replace(response_rx);
                Poll::Ready(Ok(len))
            }
            Err(error) => Poll::Ready(Err(try_send_error_to_io_error(&error))),
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
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<std::io::Result<()>> {
        let Some(receiver) = &self.receiver else {
            return self.reschedule(cx.waker().clone());
        };

        match receiver.try_recv() {
            Ok(data) => {
                self.buffer_frame(&data)?;
                self.reschedule(cx.waker().clone())
            }
            Err(error) => match error {
                TryRecvError::Empty => self.reschedule(cx.waker().clone()),
                TryRecvError::Disconnected => {
                    buf.put_slice(&self.buffer);
                    self.buffer.clear();
                    Poll::Ready(Ok(()))
                }
            },
        }
    }
}

/// Convert a [`TrySendError`] into an [`Error`] result.
fn try_send_error_to_io_error<T>(error: &TrySendError<T>) -> Error {
    match error {
        TrySendError::Full(_) => ErrorKind::WouldBlock.into(),
        TrySendError::Disconnected(_) => ErrorKind::BrokenPipe.into(),
    }
}
