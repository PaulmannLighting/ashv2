use std::fmt::Display;
use std::io;

use tokio::sync::mpsc::error::SendError;
use tokio::sync::oneshot::error::RecvError;

use crate::actor::message::Message;

/// Errors that can occur when using the proxy.
#[derive(Debug)]
pub enum Error {
    /// Error sending a message through the channel.
    SendError(SendError<Message>),
    /// Error receiving a message from the channel.
    RecvError(RecvError),
    /// I/O error.
    Io(io::Error),
}

impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::SendError(error) => write!(f, "Send error: {error}"),
            Self::RecvError(error) => write!(f, "Receive error: {error}"),
            Self::Io(error) => write!(f, "I/O error: {error}"),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::SendError(error) => Some(error),
            Self::RecvError(error) => Some(error),
            Self::Io(error) => Some(error),
        }
    }
}

impl From<SendError<Message>> for Error {
    fn from(error: SendError<Message>) -> Self {
        Self::SendError(error)
    }
}

impl From<RecvError> for Error {
    fn from(error: RecvError) -> Self {
        Self::RecvError(error)
    }
}

impl From<io::Error> for Error {
    fn from(error: io::Error) -> Self {
        Self::Io(error)
    }
}
