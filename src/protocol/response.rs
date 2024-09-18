use std::fmt::Debug;
use std::future::Future;

use crate::Error;

/// Result of handling an event.
#[derive(Debug)]
pub enum HandleResult {
    /// Indicates that the handler successfully processed the data and does not expect any more data.  
    Completed,
    /// Indicates that the handler successfully processed the data and is expecting more data.
    Continue,
    /// Indicates that the handler was unable to process the passed data and is expecting more data.
    Reject,
    /// Indicates that the handler was unable to process the passed data and cannot continue.
    Failed,
}

/// Events sent to a [`Handler`].
#[derive(Debug)]
pub enum Event<'data> {
    /// Indicates that the requested payload has been transmitted completely.
    TransmissionCompleted,
    /// Indicates that the listener has received a potential data packet that it is forwarding to the handler for processing.
    DataReceived(&'data [u8]),
}

/// Handle `ASHv2` protocol events.
pub trait Handler: Debug + Send + Sync {
    /// Handle the incoming  [`Event`] and return an appropriate [`HandleResult`].
    fn handle(&self, event: Event<'_>) -> HandleResult;

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
    /// The type to be returned by the [`Future`].
    type Result;
    /// The error type to be returned by the [`Future`].
    ///
    /// This type must be convertible from [`Error`].
    type Error: From<Error>;
}
