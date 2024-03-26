use super::response::{HandleResult, Handler};
use crate::protocol::host::command::response::Event;
use crate::Error;
use log::{error, trace};
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
    pub fn new(
        result: Arc<Mutex<Option<Result<(), Error>>>>,
        waker: Arc<Mutex<Option<Waker>>>,
    ) -> Self {
        Self {
            result,
            waker,
            transmission_complete: Arc::new(AtomicBool::new(false)),
        }
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
        trace!("Locking result.");
        if let Ok(mut result) = self.result.lock() {
            if let Some(result) = result.take() {
                return Poll::Ready(result);
            }
        }
        trace!("Releasing result.");

        trace!("Locking waker.");
        if let Ok(mut waker) = self.waker.lock() {
            waker.replace(cx.waker().clone());
        }
        trace!("Releasing waker.");

        Poll::Pending
    }
}

impl Handler<()> for ResetResponse {
    fn handle(&self, event: Event<Result<(), Error>>) -> HandleResult {
        match event {
            Event::TransmissionCompleted => {
                self.transmission_complete.store(true, SeqCst);

                trace!("Locking result.");
                let result = self.result.lock().map_or_else(
                    |_| {
                        error!("Could not lock result.");
                        HandleResult::Reject
                    },
                    |result| {
                        if result.is_some() {
                            self.wake();
                            HandleResult::Completed
                        } else {
                            HandleResult::Continue
                        }
                    },
                );
                trace!("Releasing result.");
                result
            }
            Event::DataReceived(data) => {
                trace!("Locking result.");
                let result = self.result.lock().map_or_else(
                    |_| {
                        error!("Could not lock result.");
                        HandleResult::Reject
                    },
                    |mut result| {
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
                    },
                );
                trace!("Releasing result.");
                result
            }
        }
    }

    fn abort(&self, error: Error) {
        trace!("Locking result.");
        if let Ok(mut result) = self.result.lock() {
            result.replace(Err(error));
        }
        trace!("Releasing result.");
    }

    fn wake(&self) {
        trace!("Locking waker.");
        if let Ok(mut waker) = self.waker.lock() {
            if let Some(waker) = waker.take() {
                waker.wake();
            }
        }
        trace!("Releasing waker.");
    }
}
