use std::fmt::Display;

use tokio::sync::mpsc::error::SendError;
use tokio::task::JoinError;

use crate::actor::message::Message;

#[derive(Debug)]
pub enum Error {
    Send(SendError<Message>),
    Join(JoinError),
}

impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Send(error) => error.fmt(f),
            Self::Join(error) => error.fmt(f),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Send(error) => Some(error),
            Self::Join(error) => Some(error),
        }
    }
}

impl From<SendError<Message>> for Error {
    fn from(error: SendError<Message>) -> Self {
        Self::Send(error)
    }
}

impl From<JoinError> for Error {
    fn from(error: JoinError) -> Self {
        Self::Join(error)
    }
}
