use crate::Error;
use std::fmt::Debug;
use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll, Waker};

#[derive(Clone, Debug)]
pub struct Inner<Request, Response>
where
    Request: Debug + Clone,
{
    request: Request,
    result: Arc<Mutex<Option<Result<Response, Error>>>>,
    waker: Arc<Mutex<Option<Waker>>>,
}

impl<Request, Response> Inner<Request, Response>
where
    Request: Debug + Clone,
{
    #[must_use]
    pub fn new(request: Request) -> Self {
        Self {
            request,
            result: Arc::new(Mutex::new(None)),
            waker: Arc::new(Mutex::new(None)),
        }
    }

    #[must_use]
    pub const fn request(&self) -> &Request {
        &self.request
    }

    pub fn resolve(&self, result: Result<Response, Error>) {
        if let Ok(mut lock) = self.result.lock() {
            lock.replace(result);

            if let Ok(mut waker) = self.waker.lock() {
                if let Some(waker) = waker.take() {
                    waker.wake();
                }
            }
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
