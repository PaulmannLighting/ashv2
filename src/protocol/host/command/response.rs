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
pub enum Event<T>
where
    T: Debug,
{
    TransmissionCompleted,
    DataReceived(T),
}

pub trait Handler<T>: Debug + Send + Sync
where
    T: Debug + Send + Sync,
{
    fn handle(&self, event: Event<Result<T, Error>>) -> HandleResult;
    fn abort(&self, error: Error);
    fn wake(&self);
}

pub trait Response:
    Handler<Arc<[u8]>> + Future<Output = Result<Self::Result, Self::Error>>
where
    Self::Error: From<Error>,
{
    type Result;
    type Error;
}
