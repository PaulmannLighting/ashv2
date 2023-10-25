use crate::protocol::ash_chunks::AshChunks;
use crate::Error;
use itertools::IntoChunks;
use std::future::Future;
use std::iter::Copied;
use std::pin::Pin;
use std::slice::Iter;
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll, Waker};

#[derive(Debug)]
pub struct Transaction {
    request: Arc<[u8]>,
    result: Arc<Mutex<Option<Result<Arc<[u8]>, Error>>>>,
    waker: Arc<Mutex<Option<Waker>>>,
}

impl Transaction {
    #[must_use]
    pub fn new(request: Arc<[u8]>) -> Self {
        Self {
            request,
            result: Arc::new(Mutex::new(None)),
            waker: Arc::new(Mutex::new(None)),
        }
    }

    #[must_use]
    pub fn request(&self) -> &[u8] {
        &self.request
    }

    pub fn chunks(&mut self) -> Result<IntoChunks<Copied<Iter<'_, u8>>>, Error> {
        self.request.iter().copied().ash_chunks()
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
        Self::new(bytes.into())
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
