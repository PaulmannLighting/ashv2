use crate::Request;
use std::io::ErrorKind;
use std::sync::mpsc::{sync_channel, Receiver, SyncSender};
use std::sync::{Arc, Mutex};
use std::task::{Poll, Waker};
use std::thread::spawn;
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};

type SharedResult<T> = Arc<Mutex<Option<T>>>;

/// A framed asynchronous `ASHv2` host.
#[derive(Debug)]
pub struct AshFramed<const BUF_SIZE: usize> {
    sender: SyncSender<Request>,
    sent_bytes: SharedResult<std::io::Result<usize>>,
    receiver: SharedResult<Receiver<Box<[u8]>>>,
    result: SharedResult<std::io::Result<Box<[u8]>>>,
}

impl<const BUF_SIZE: usize> AshFramed<BUF_SIZE> {
    /// Create a new `AshFramed` instance.
    #[must_use]
    pub fn new(sender: SyncSender<Request>) -> Self {
        Self {
            sender,
            sent_bytes: Arc::new(Mutex::new(None)),
            receiver: Arc::new(Mutex::new(None)),
            result: Arc::new(Mutex::new(None)),
        }
    }
}

impl<const BUF_SIZE: usize> AsyncWrite for AshFramed<BUF_SIZE> {
    fn poll_write(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &[u8],
    ) -> Poll<std::io::Result<usize>> {
        let result = self
            .sent_bytes
            .lock()
            .expect("sent_bytes mutex poisoned")
            .take();

        if let Some(result) = result {
            return Poll::Ready(result);
        }

        spawn_writer::<BUF_SIZE>(
            self.sender.clone(),
            cx.waker().clone(),
            buf.to_vec().into_boxed_slice(),
            self.sent_bytes.clone(),
            self.receiver.clone(),
        );

        Poll::Pending
    }

    fn poll_flush(
        self: std::pin::Pin<&mut Self>,
        _cx: &mut std::task::Context<'_>,
    ) -> Poll<std::io::Result<()>> {
        Poll::Ready(Ok(()))
    }

    fn poll_shutdown(
        self: std::pin::Pin<&mut Self>,
        _cx: &mut std::task::Context<'_>,
    ) -> Poll<std::io::Result<()>> {
        self.sent_bytes
            .lock()
            .expect("sent_bytes mutex poisoned")
            .take();
        self.receiver
            .lock()
            .expect("receiver mutex poisoned")
            .take();
        Poll::Ready(Ok(()))
    }
}

impl<const BUF_SIZE: usize> AsyncRead for AshFramed<BUF_SIZE> {
    fn poll_read(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<std::io::Result<()>> {
        let receiver = self
            .receiver
            .lock()
            .expect("receiver mutex poisoned")
            .take();

        if let Some(receiver) = receiver {
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

fn spawn_writer<const BUF_SIZE: usize>(
    sender: SyncSender<Request>,
    waker: Waker,
    payload: Box<[u8]>,
    sent_bytes: SharedResult<std::io::Result<usize>>,
    receiver: SharedResult<Receiver<Box<[u8]>>>,
) {
    spawn(move || {
        let len = payload.len();
        let (response_tx, response_rx) = sync_channel(BUF_SIZE);
        let request = Request::new(payload, response_tx);
        let result = sender
            .send(request)
            .map(|()| len)
            .map_err(|_| ErrorKind::BrokenPipe.into());
        sent_bytes
            .lock()
            .expect("sent_bytes mutex poisoned")
            .replace(result);
        receiver
            .lock()
            .expect("receiver mutex poisoned")
            .replace(response_rx);
        waker.wake();
    });
}

fn spawn_reader(
    receiver: Receiver<Box<[u8]>>,
    waker: Waker,
    state: SharedResult<std::io::Result<Box<[u8]>>>,
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
