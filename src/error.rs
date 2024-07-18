use std::fmt::{Display, Formatter};
use std::sync::Arc;

pub mod frame;

/// Possible error states during `ASHv2` transactions.
#[derive(Clone, Debug)]
pub enum Error {
    Frame(frame::Error),
    Io(Arc<std::io::Error>),
    SerialConnectionError(serialport::Error),
    CannotFindViableChunkSize(usize),
    MaxRetransmitsExceeded,
    InitializationFailed,
    Terminated,
    Aborted,
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Frame(error) => write!(f, "{error}"),
            Self::Io(error) => write!(f, "{error}"),
            Self::SerialConnectionError(error) => write!(f, "{error}"),
            Self::CannotFindViableChunkSize(size) => {
                write!(f, "Cannot find viable chunk size for {size} bytes")
            }
            Self::MaxRetransmitsExceeded => write!(f, "Maximum amount of retransmits exceeded"),
            Self::InitializationFailed => write!(f, "ASH protocol initialization failed"),
            Self::Terminated => write!(f, "Worker terminated"),
            Self::Aborted => write!(f, "Transaction aborted"),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Frame(error) => Some(error),
            Self::Io(error) => Some(error),
            Self::SerialConnectionError(error) => Some(error),
            _ => None,
        }
    }
}

impl From<frame::Error> for Error {
    fn from(error: frame::Error) -> Self {
        Self::Frame(error)
    }
}

impl From<std::io::Error> for Error {
    fn from(error: std::io::Error) -> Self {
        Self::Io(Arc::new(error))
    }
}

impl From<serialport::Error> for Error {
    fn from(error: serialport::Error) -> Self {
        Self::SerialConnectionError(error)
    }
}
