use crate::Error;
use std::fmt::Debug;

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

pub trait Response<T>: Debug + Send + Sync
where
    T: Debug + Send + Sync,
{
    fn handle(&self, event: Event<Result<T, Error>>) -> HandleResult;
    fn abort(&self, error: Error);
    fn wake(&self);
}
