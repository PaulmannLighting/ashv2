use crate::Error;
use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll, Waker};

type ResultType = Result<Arc<[u8]>, Error>;

#[derive(Clone, Debug)]
pub enum Request {
    Data(Arc<[u8]>),
    Terminate,
}

#[derive(Clone, Debug)]
pub struct Transaction {
    request: Request,
    result: Arc<Mutex<Option<ResultType>>>,
    waker: Arc<Mutex<Option<Waker>>>,
}

impl Transaction {
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

    pub fn resolve(&self, result: Result<Arc<[u8]>, Error>) {
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

impl From<&[u8]> for Transaction {
    fn from(bytes: &[u8]) -> Self {
        Self::new(Request::Data(bytes.into()))
    }
}

impl Future for Transaction {
    type Output = Result<Arc<[u8]>, Error>;

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
