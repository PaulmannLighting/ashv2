use crate::protocol::host::command::Command;
use crate::Error;
use std::fmt::Debug;
use std::future::Future;
use std::sync::mpsc::SendError;
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
    Future<Output = Result<Self::Result, Self::Error>> + Handler<Arc<[u8]>>
where
    Self::Error: From<Error> + From<SendError<Command>>,
{
    type Result;
    type Error;
}
