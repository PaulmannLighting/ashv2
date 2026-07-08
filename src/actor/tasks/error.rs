use std::fmt::Display;

use tokio::sync::mpsc::error::SendError;

use crate::actor::message::Message;

/// Error returned when requesting actor termination fails.
#[derive(Debug)]
pub enum Error {
    /// Sending the termination message to the transmitter failed.
    Send(SendError<Message>),
}

impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Send(error) => error.fmt(f),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Send(error) => Some(error),
        }
    }
}

impl From<SendError<Message>> for Error {
    fn from(error: SendError<Message>) -> Self {
        Self::Send(error)
    }
}
