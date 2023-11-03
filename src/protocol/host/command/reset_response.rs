use super::response::{HandleResult, Response};
use crate::protocol::host::command::response::Event;
use crate::Error;
use log::error;
use std::future::Future;
use std::pin::Pin;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering::SeqCst;
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll, Waker};

#[derive(Clone, Debug)]
pub struct ResetResponse {
    result: Arc<Mutex<Option<Result<(), Error>>>>,
    waker: Arc<Mutex<Option<Waker>>>,
    transmission_complete: Arc<AtomicBool>,
}

impl ResetResponse {
    #[must_use]
    pub fn new() -> Self {
        Self {
            result: Arc::new(Mutex::new(None)),
            waker: Arc::new(Mutex::new(None)),
            transmission_complete: Arc::new(AtomicBool::new(false)),
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

impl Default for ResetResponse {
    fn default() -> Self {
        Self::new()
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

impl Response<()> for ResetResponse {
    fn handle(&self, event: Event<Result<(), Error>>) -> HandleResult {
        match event {
            Event::TransmissionCompleted => {
                self.transmission_complete.store(true, SeqCst);

                if let Ok(result) = self.result.lock() {
                    if result.is_some() {
                        self.wake();
                        HandleResult::Completed
                    } else {
                        HandleResult::Continue
                    }
                } else {
                    error!("Could not lock result.");
                    HandleResult::Reject
                }
            }
            Event::DataReceived(data) => {
                if let Ok(mut result) = self.result.lock() {
                    if result.is_some() {
                        HandleResult::Reject
                    } else {
                        result.replace(data);

                        if self.transmission_complete.load(SeqCst) {
                            self.wake();
                            HandleResult::Completed
                        } else {
                            HandleResult::Continue
                        }
                    }
                } else {
                    error!("Could not lock result.");
                    HandleResult::Reject
                }
            }
        }
    }

    fn abort(&self, error: Error) {
        if let Ok(mut result) = self.result.lock() {
            result.replace(Err(error));
        }
    }
}
