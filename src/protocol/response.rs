use crate::Error;
use std::fmt::Debug;
use std::future::Future;
use std::sync::Arc;

#[derive(Debug)]
pub enum HandleResult {
    Completed,
    Continue,
    Failed,
    Reject,
}

#[derive(Debug)]
pub enum Event {
    TransmissionCompleted,
    DataReceived(Result<Arc<[u8]>, Error>),
}

pub trait Handler: Debug + Send + Sync {
    fn handle(&self, event: Event) -> HandleResult;
    fn abort(&self, error: Error);
    fn wake(&self);
}

pub trait Response: Future<Output = Result<Self::Result, Self::Error>> + Handler
where
    Self::Error: From<Error>,
{
    type Result;
    type Error;
}
