use std::fmt::{Display, Formatter};
use std::sync::mpsc::SendError;
use std::sync::{Arc, PoisonError};

#[derive(Debug)]
pub enum Error {
    InvalidHeader(Option<u8>),
    BufferTooSmall { expected: usize, found: usize },
    BufferTooLarge { expected: usize, found: usize },
    InvalidBufferSize { expected: usize, found: usize },
    TooMuchData(usize),
    TooFewData(usize),
    CannotFindViableChunkSize(usize),
    Io(std::io::Error),
    Terminated,
    LockError(Arc<dyn std::error::Error + Send + Sync>),
    SendError(Arc<dyn std::error::Error + Send + Sync>),
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidHeader(header) => match header {
                Some(id) => write!(f, "Invalid header ID: {}.", *id),
                None => write!(f, "No header received."),
            },
            Self::BufferTooSmall { expected, found } => {
                write!(
                    f,
                    "Buffer too small. Expected at least {expected} bytes but found {found} bytes."
                )
            }
            Self::BufferTooLarge { expected, found } => {
                write!(
                    f,
                    "Buffer too large. Expected at most {expected} bytes but found {found} bytes."
                )
            }
            Self::InvalidBufferSize { expected, found } => write!(
                f,
                "Invalid buffer size. Expected {expected} bytes, but found {found} bytes."
            ),
            Self::TooMuchData(size) => write!(f, "Too much data: {size} bytes"),
            Self::TooFewData(size) => write!(f, "Too few data: {size} bytes"),
            Self::CannotFindViableChunkSize(size) => {
                write!(f, "Cannot find viable chunk size for {size} bytes")
            }
            Self::Io(error) => write!(f, "{error}"),
            Self::Terminated => write!(f, "terminated"),
            Self::LockError(error) | Self::SendError(error) => write!(f, "{error}"),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(error) => Some(error),
            Self::LockError(error) | Self::SendError(error) => Some(error),
            _ => None,
        }
    }
}

impl From<Error> for std::io::Error {
    fn from(error: Error) -> Self {
        match error {
            Error::BufferTooSmall { .. }
            | Error::BufferTooLarge { .. }
            | Error::InvalidBufferSize { .. } => Self::new(std::io::ErrorKind::Other, error),
            Error::Io(error) => error,
            error => Self::new(std::io::ErrorKind::InvalidData, error),
        }
    }
}

impl From<std::io::Error> for Error {
    fn from(error: std::io::Error) -> Self {
        Self::Io(error)
    }
}

impl<T> From<PoisonError<T>> for Error
where
    T: Send + Sync + 'static,
{
    fn from(error: PoisonError<T>) -> Self {
        Self::LockError(Arc::new(error))
    }
}

impl<T> From<SendError<T>> for Error
where
    T: Send + Sync + 'static,
{
    fn from(error: SendError<T>) -> Self {
        Self::SendError(Arc::new(error))
    }
}
