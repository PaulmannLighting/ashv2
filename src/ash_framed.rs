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
    state: Arc<Mutex<SharedState>>,
}

impl<const BUF_SIZE: usize> AshFramed<BUF_SIZE> {
    /// Create a new `AshFramed` instance.
    #[must_use]
    pub fn new(sender: SyncSender<Request>) -> Self {
        Self {
            sender,
            state: Arc::new(Mutex::new(SharedState::default())),
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
            .state
            .lock()
            .expect("sent_bytes mutex poisoned")
            .sent_bytes
            .take();

        if let Some(result) = result {
            return Poll::Ready(result);
        }

        if self
            .state
            .lock()
            .expect("sent_bytes mutex poisoned")
            .sending
        {
            return Poll::Pending;
        }

        self.state
            .lock()
            .expect("sent_bytes mutex poisoned")
            .sending = true;
        spawn_writer::<BUF_SIZE>(
            self.sender.clone(),
            cx.waker().clone(),
            buf.to_vec().into_boxed_slice(),
            self.state.clone(),
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
        self.state
            .lock()
            .expect("sent_bytes mutex poisoned")
            .reset();
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
            .state
            .lock()
            .expect("receiver mutex poisoned")
            .receiver
            .take();

        if let Some(receiver) = receiver {
            spawn_reader(receiver, cx.waker().clone(), self.state.clone());
        }

        self.state
            .lock()
            .expect("result mutex poisoned")
            .result
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
    state: Arc<Mutex<SharedState>>,
) {
    spawn(move || {
        let len = payload.len();
        let (response_tx, response_rx) = sync_channel(BUF_SIZE);
        let request = Request::new(payload, response_tx);
        let result = sender
            .send(request)
            .map(|()| len)
            .map_err(|_| ErrorKind::BrokenPipe.into());
        let mut lock = state.lock().expect("sent_bytes mutex poisoned");
        lock.sent_bytes.replace(result);
        lock.receiver.replace(response_rx);
        lock.sending = false;
        drop(lock);
        waker.wake();
    });
}

fn spawn_reader(receiver: Receiver<Box<[u8]>>, waker: Waker, state: Arc<Mutex<SharedState>>) {
    spawn(move || {
        let result = receiver.recv();
        state
            .lock()
            .expect("result mutex poisoned")
            .result
            .replace(result.map_err(|_| ErrorKind::BrokenPipe.into()));
        waker.wake();
    });
}

#[derive(Debug, Default)]
struct SharedState {
    sending: bool,
    sent_bytes: Option<std::io::Result<usize>>,
    receiver: Option<Receiver<Box<[u8]>>>,
    result: Option<std::io::Result<Box<[u8]>>>,
}

impl SharedState {
    fn reset(&mut self) {
        self.sending = false;
        self.sent_bytes = None;
        self.receiver = None;
        self.result = None;
    }
}
