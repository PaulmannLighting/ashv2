use std::fmt::Debug;
use std::future::Future;

use crate::Error;

/// Result of handling an event.
///
/// This enum is returned from [`Handler::handle`] to indicate the outcome of handling the respective event.
///
/// * [`HandleResult::Completed`] indicates that the handler successfully processed the data and does not expect any more data.  
/// * [`HandleResult::Continue`] indicates that the handler successfully processed the data and is expecting more data.
/// * [`HandleResult::Failed`] indicates that the handler was unable to process the passed data and cannot continue.
/// * [`HandleResult::Reject`] indicates that the handler was unable to process the passed data and is expecting more data.
#[derive(Debug)]
pub enum HandleResult {
    Completed,
    Continue,
    Failed,
    Reject,
}

/// Events sent to a [`Handler`].
///
/// * [`Event::TransmissionCompleted`] indicates that the requested payload has been transmitted completely.
/// * [`Event::DataReceived`] indicates that the listener has received a potential data packet that it is forwarding to the handler for processing.
#[derive(Debug)]
pub enum Event<'data> {
    TransmissionCompleted,
    DataReceived(&'data [u8]),
}

/// Handle `ASHv2` protocol events.
pub trait Handler: Debug + Send + Sync {
    /// Handle the incoming  [`Event`] and return an appropriate [`HandleResult`].
    fn handle(&self, event: Event) -> HandleResult;

    /// Abort the current transaction, resulting in an erroneous state.
    fn abort(&self, error: Error);

    /// Wake the underlying [`Waker`](std::task::Waker) to complete the respective [`Future`].
    fn wake(&self);
}

/// A response to a request sent to the NCP.
///
/// This is a composite trait consisting of a [`Future`] and a [`Handler`] implementation.
/// The Future must output [`Result<Self::Result, Self::Error>`].
pub trait Response: Future<Output = Result<Self::Result, Self::Error>> + Handler {
    type Result;
    type Error: From<Error>;
}
