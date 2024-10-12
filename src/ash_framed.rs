mod shared_state;

use crate::Request;
use shared_state::SharedState;
use std::io::ErrorKind;
use std::pin::Pin;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering::Relaxed;
use std::sync::mpsc::{sync_channel, Receiver, SyncSender, TrySendError};
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll};
use std::thread::{sleep, spawn, JoinHandle};
use std::time::Duration;
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};

const BURST: Duration = Duration::from_millis(100);

/// A framed asynchronous `ASHv2` host.
#[derive(Debug)]
pub struct AshFramed<const BUF_SIZE: usize> {
    sender: SyncSender<Request>,
    running: Arc<AtomicBool>,
    state: Arc<Mutex<SharedState>>,
    receiver: Option<JoinHandle<()>>,
}

impl<const BUF_SIZE: usize> AshFramed<BUF_SIZE> {
    /// Create a new `AshFramed` instance.
    #[must_use]
    pub fn new(sender: SyncSender<Request>) -> Self {
        let running = Arc::new(AtomicBool::new(true));
        let state = Arc::new(Mutex::new(SharedState::default()));
        let receiver = spawn_reader(running.clone(), state.clone());
        Self {
            sender,
            running,
            state,
            receiver: Some(receiver),
        }
    }
}

impl<const BUF_SIZE: usize> Drop for AshFramed<BUF_SIZE> {
    fn drop(&mut self) {
        self.running.store(false, Relaxed);

        if let Some(receiver) = self.receiver.take() {
            receiver.join().expect("thread panicked");
        }
    }
}

impl<const BUF_SIZE: usize> AsyncWrite for &AshFramed<BUF_SIZE> {
    fn poll_write(
        self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<std::io::Result<usize>> {
        let len = buf.len();
        let (response_tx, response_rx) = sync_channel(BUF_SIZE);
        let request = Request::new(buf.as_ref().into(), response_tx);

        match self.sender.try_send(request) {
            Ok(()) => {
                self.state
                    .lock()
                    .expect("mutex poisoned")
                    .receiver
                    .replace(response_rx);
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

    fn poll_shutdown(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        self.state.lock().expect("mutex poisoned").reset();
        Poll::Ready(Ok(()))
    }
}

impl<const BUF_SIZE: usize> AsyncRead for &AshFramed<BUF_SIZE> {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<std::io::Result<()>> {
        let mut lock = self.state.lock().expect("mutex poisoned");
        lock.result.take().map_or_else(
            || {
                lock.waker.replace(cx.waker().clone());
                Poll::Pending
            },
            |result| match result {
                Ok(data) => {
                    buf.put_slice(&data);
                    Poll::Ready(Ok(()))
                }
                Err(error) => Poll::Ready(Err(error)),
            },
        )
    }
}

fn spawn_reader(running: Arc<AtomicBool>, state: Arc<Mutex<SharedState>>) -> JoinHandle<()> {
    spawn(move || {
        while running.load(Relaxed) {
            let receiver = state.lock().expect("mutex poisoned").receiver.take();

            if let Some(receiver) = receiver {
                receive_loop(&receiver, &state);
            } else {
                sleep(BURST);
                continue;
            }
        }
    })
}

fn receive_loop(receiver: &Receiver<Box<[u8]>>, state: &Arc<Mutex<SharedState>>) {
    loop {
        if let Ok(data) = receiver.recv() {
            state
                .lock()
                .expect("mutex poisoned")
                .buffer
                .extend_from_slice(&data);
        } else {
            let mut lock = state.lock().expect("mutex poisoned");

            if lock.buffer.is_empty() {
                lock.result.replace(Err(ErrorKind::UnexpectedEof.into()));
            } else {
                let result = Ok(lock.buffer.clone().into_boxed_slice());
                lock.buffer.clear();
                lock.result.replace(result);
            }

            if let Some(waker) = lock.waker.take() {
                waker.wake();
            }

            return;
        }
    }
}
