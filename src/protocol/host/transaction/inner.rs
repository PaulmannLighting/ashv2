use crate::Error;
use std::fmt::Debug;
use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll, Waker};

#[derive(Clone, Debug)]
pub struct Inner<I, O>
where
    I: Debug + Clone,
{
    request: I,
    result: Arc<Mutex<Option<Result<O, Error>>>>,
    waker: Arc<Mutex<Option<Waker>>>,
}

impl<I, O> Inner<I, O>
where
    I: Debug + Clone,
{
    #[must_use]
    pub fn new(request: I) -> Self {
        Self {
            request,
            result: Arc::new(Mutex::new(None)),
            waker: Arc::new(Mutex::new(None)),
        }
    }

    #[must_use]
    pub const fn request(&self) -> &I {
        &self.request
    }

    pub fn resolve(self, result: Result<O, Error>) {
        self.result
            .lock()
            .expect("Could not lock result.")
            .replace(result);

        if let Some(waker) = self.waker.lock().expect("Could not lock waker.").take() {
            waker.wake();
        }
    }
}

impl<Request, Response> Future for Inner<Request, Response>
where
    Request: Debug + Clone,
{
    type Output = Result<Response, Error>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        if let Ok(mut result) = self.result.lock() {
            if let Some(result) = result.take() {
                return Poll::Ready(result);
            }
        }

        if let Ok(mut waker) = self.waker.lock() {
            waker.replace(cx.waker().clone());
        }

        Poll::Pending
    }
}
