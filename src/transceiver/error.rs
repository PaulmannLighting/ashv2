use std::fmt::Display;

pub type Result<T> = std::result::Result<T, Error>;

/// A transceiver-internal error.
#[derive(Debug)]
pub enum Error {
    Io(std::io::Error),
    EnteredFailedState,
}

impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(error) => write!(f, "IO error: {error}"),
            Self::EnteredFailedState => write!(f, "Entered failed state"),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(error) => Some(error),
            Self::EnteredFailedState => None,
        }
    }
}

impl From<std::io::Error> for Error {
    fn from(error: std::io::Error) -> Self {
        Self::Io(error)
    }
}
