mod communicate_async;
mod communicate_sync;

use crate::request::Request;
pub use communicate_async::CommunicateAsync;
pub use communicate_sync::CommunicateSync;
use std::sync::mpsc::Sender;

/// A host controller to communicate with an NCP via the `ASHv2` protocol.
#[derive(Debug)]
pub struct Host {
    command: Sender<Request>,
}

impl Host {
    /// Creates and starts the host.
    #[must_use]
    pub const fn new(command: Sender<Request>) -> Self {
        Self { command }
    }
}

impl From<Sender<Request>> for Host {
    fn from(command: Sender<Request>) -> Self {
        Self::new(command)
    }
}
