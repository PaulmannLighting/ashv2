use std::future::Future;
use std::io::{Error, ErrorKind, Result};
use std::pin::Pin;
use std::sync::mpsc::{Receiver, TryRecvError};
use std::task::{Context, Poll, Waker};

#[derive(Debug)]
pub struct Response {
    receiver: Option<Receiver<Result<Box<[u8]>>>>,
    waker: Option<Waker>,
}

impl Response {
    #[must_use]
    pub const fn new(receiver: Receiver<Result<Box<[u8]>>>) -> Self {
        Self {
            receiver: Some(receiver),
            waker: None,
        }
    }

    pub fn failed() -> Self {
        Self {
            receiver: None,
            waker: None,
        }
    }
}

impl Future for Response {
    type Output = Result<Box<[u8]>>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        if let Some(receiver) = self.receiver.as_mut() {
            match receiver.try_recv() {
                Ok(payload) => Poll::Ready(payload),
                Err(error) => match error {
                    TryRecvError::Empty => {
                        self.waker.get_or_insert(cx.waker().clone()).wake_by_ref();
                        Poll::Pending
                    }
                    TryRecvError::Disconnected => Poll::Ready(Err(Error::new(
                        ErrorKind::BrokenPipe,
                        "ASHv2 response channel disconnected.",
                    ))),
                },
            }
        } else {
            Poll::Ready(Err(Error::new(
                ErrorKind::BrokenPipe,
                "ASHv2 failed to send request.",
            )))
        }
    }
}
