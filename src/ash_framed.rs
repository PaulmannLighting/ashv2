use crate::Request;
use std::io::ErrorKind;
use std::sync::mpsc::{sync_channel, Receiver, SyncSender};
use std::sync::{Arc, Mutex};
use std::task::{Poll, Waker};
use std::thread::spawn;
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};

/// A framed asynchronous `ASHv2` host.
#[derive(Debug)]
pub struct AshFramed<const BUF_SIZE: usize> {
    sender: SyncSender<Request>,
    receiver: Option<Receiver<Box<[u8]>>>,
    result: Arc<Mutex<Option<std::io::Result<Box<[u8]>>>>>,
}

impl<const BUF_SIZE: usize> AshFramed<BUF_SIZE> {
    /// Create a new `AshFramed` instance.
    #[must_use]
    pub fn new(sender: SyncSender<Request>) -> Self {
        Self {
            sender,
            receiver: None,
            result: Arc::new(Mutex::new(None)),
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

impl<const BUF_SIZE: usize> AsyncRead for AshFramed<BUF_SIZE> {
    fn poll_read(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<std::io::Result<()>> {
        if let Some(receiver) = self.receiver.take() {
            spawn_reader(receiver, cx.waker().clone(), self.result.clone());
        }

        self.result
            .lock()
            .expect("result mutex poisoned")
            .take()
            .map_or_else(
                || Poll::Pending,
                |result| match result {
                    Ok(data) => {
                        buf.put_slice(&data);
                        Poll::Ready(Ok(()))
                    }
                    Err(e) => Poll::Ready(Err(e)),
                },
            )
    }
}

fn spawn_reader(
    receiver: Receiver<Box<[u8]>>,
    waker: Waker,
    state: Arc<Mutex<Option<std::io::Result<Box<[u8]>>>>>,
) {
    spawn(move || {
        if let Ok(payload) = receiver.recv() {
            state
                .lock()
                .expect("result mutex poisoned")
                .replace(Ok(payload));
        } else {
            state
                .lock()
                .expect("result mutex poisoned")
                .replace(Err(ErrorKind::BrokenPipe.into()));
        }
        waker.wake();
    });
}
