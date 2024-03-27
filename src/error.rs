mod frame;

#[allow(clippy::module_name_repetitions)]
pub use frame::Error as FrameError;
use std::fmt::{Display, Formatter};
use std::sync::mpsc::SendError;
use std::sync::{Arc, PoisonError};

#[derive(Clone, Debug)]
pub enum Error {
    Frame(FrameError),
    Io(Arc<std::io::Error>),
    LockError(Arc<dyn std::error::Error + Send + Sync + 'static>),
    SendError(Arc<dyn std::error::Error + Send + Sync + 'static>),
    SerialConnectionError(serialport::Error),
    CannotFindViableChunkSize(usize),
    MaxRetransmitsExceeded,
    WorkerNotRunning,
    InitializationFailed,
    Terminated,
    Aborted,
    AlreadyRunning,
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Frame(error) => write!(f, "{error}"),
            Self::Io(error) => write!(f, "{error}"),
            Self::LockError(error) | Self::SendError(error) => write!(f, "{error}"),
            Self::SerialConnectionError(error) => write!(f, "{error}"),
            Self::CannotFindViableChunkSize(size) => {
                write!(f, "Cannot find viable chunk size for {size} bytes")
            }
            Self::MaxRetransmitsExceeded => write!(f, "Maximum amount of retransmits exceeded."),
            Self::WorkerNotRunning => write!(f, "Worker is not running."),
            Self::InitializationFailed => write!(f, "ASH protocol initialization failed."),
            Self::Terminated => write!(f, "Worker terminated."),
            Self::Aborted => write!(f, "Transaction aborted."),
            Self::AlreadyRunning => write!(f, "Host is already running."),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Frame(error) => Some(error),
            Self::Io(error) => Some(error),
            Self::LockError(error) | Self::SendError(error) => Some(error),
            Self::SerialConnectionError(error) => Some(error),
            _ => None,
        }
    }
}

impl From<FrameError> for Error {
    fn from(error: FrameError) -> Self {
        Self::Frame(error)
    }
}

impl From<std::io::Error> for Error {
    fn from(error: std::io::Error) -> Self {
        Self::Io(Arc::new(error))
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

impl From<serialport::Error> for Error {
    fn from(error: serialport::Error) -> Self {
        Self::SerialConnectionError(error)
    }
}
