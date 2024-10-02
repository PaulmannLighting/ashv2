use crate::request::Request;
use std::future::Future;
use std::io::{Error, ErrorKind, Result};
use std::pin::Pin;
use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll, Waker};
use std::thread::spawn;

#[derive(Debug)]
pub struct AsyncRequest {
    shared_state: Arc<Mutex<SharedState>>,
}

impl AsyncRequest {
    #[must_use]
    pub fn new(sender: Sender<Request>, payload: &[u8]) -> Self {
        let shared_state = Arc::new(Mutex::new(SharedState {
            output: None,
            waker: None,
        }));

        let (request, response) = Request::new(payload.into());
        let thread_shared_state = shared_state.clone();

        spawn(move || {
            if sender.send(request).is_err() {
                thread_shared_state
                    .lock()
                    .expect("Mutex is poisoned.")
                    .set_output(Err(Error::new(
                        ErrorKind::BrokenPipe,
                        "ASHv2 failed to send request.",
                    )));
                return;
            }

            if let Ok(payload) = response.recv() {
                thread_shared_state
                    .lock()
                    .expect("Mutex is poisoned.")
                    .set_output(payload);
            } else {
                thread_shared_state
                    .lock()
                    .expect("Mutex is poisoned.")
                    .set_output(Err(Error::new(
                        ErrorKind::BrokenPipe,
                        "ASHv2 response channel disconnected.",
                    )));
            }
        });

        Self { shared_state }
    }
}

impl Future for AsyncRequest {
    type Output = Result<Box<[u8]>>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let mut shared_state = self.shared_state.lock().expect("Mutex is poisoned.");

        shared_state.output.take().map_or_else(
            || {
                shared_state.waker = Some(cx.waker().clone());
                Poll::Pending
            },
            Poll::Ready,
        )
    }
}

#[derive(Debug)]
struct SharedState {
    output: Option<Result<Box<[u8]>>>,
    waker: Option<Waker>,
}

impl SharedState {
    fn set_output(&mut self, output: Result<Box<[u8]>>) {
        self.output.replace(output);

        if let Some(waker) = self.waker.take() {
            waker.wake();
        }
    }
}
