use super::response::{HandleResult, Handler};
use crate::protocol::host::command::response::Event;
use crate::Error;
use log::error;
use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll, Waker};

#[derive(Clone, Debug)]
pub struct ResetResponse {
    result: Arc<Mutex<Option<Result<(), Error>>>>,
    waker: Arc<Mutex<Option<Waker>>>,
}

impl ResetResponse {
    #[must_use]
    pub fn new(
        result: Arc<Mutex<Option<Result<(), Error>>>>,
        waker: Arc<Mutex<Option<Waker>>>,
    ) -> Self {
        Self { result, waker }
    }
}

impl Default for ResetResponse {
    fn default() -> Self {
        Self::new(Arc::new(Mutex::new(None)), Arc::new(Mutex::new(None)))
    }
}

impl Future for ResetResponse {
    type Output = Result<(), Error>;

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

impl Handler<()> for ResetResponse {
    fn handle(&self, event: Event<Result<(), Error>>) -> HandleResult {
        match event {
            Event::TransmissionCompleted => self.result.lock().map_or_else(
                |_| {
                    error!("Could not lock result.");
                    HandleResult::Reject
                },
                |mut result| {
                    if result.is_none() {
                        result.replace(Ok(()));
                    }

                    HandleResult::Completed
                },
            ),
            Event::DataReceived(_) => {
                error!("Received data. Discarding.");

                self.result.lock().map_or_else(
                    |_| {
                        error!("Could not lock result.");
                        HandleResult::Reject
                    },
                    |mut result| {
                        result.replace(Err(Error::Aborted));
                        HandleResult::Reject
                    },
                )
            }
        }
    }

    fn abort(&self, error: Error) {
        if let Ok(mut result) = self.result.lock() {
            result.replace(Err(error));
        }
    }

    fn wake(&self) {
        if let Ok(mut waker) = self.waker.lock() {
            if let Some(waker) = waker.take() {
                waker.wake();
            }
        }
    }
}
