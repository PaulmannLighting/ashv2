mod async_ash;
mod sync_ash;

use crate::request::Request;
pub use async_ash::AsyncAsh;
use std::sync::mpsc::{SendError, Sender, SyncSender};
pub use sync_ash::SyncAsh;

/// A trait to identify types that can be used as `ASHv2` hosts.
pub trait Host {
    /// Send a request to the transceiver.
    fn send(&self, request: Request) -> Result<(), SendError<Request>>;
}

impl Host for Sender<Request> {
    fn send(&self, request: Request) -> Result<(), SendError<Request>> {
        Self::send(self, request)
    }
}

impl Host for SyncSender<Request> {
    fn send(&self, request: Request) -> Result<(), SendError<Request>> {
        Self::send(self, request)
    }
}
