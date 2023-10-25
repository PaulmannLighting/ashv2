use std::fmt::{Display, Formatter};

#[derive(Debug)]
pub enum Error {
    MissingHeader,
    InvalidHeader(u8),
    BufferTooSmall(usize),
    InvalidBufferSize { expected: usize, found: usize },
    NoData,
    TooMuchData(usize),
    TooFewData(usize),
    CannotFindViableChunkSize(usize),
    Io(std::io::Error),
    Terminated,
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingHeader => write!(f, "Missing header."),
            Self::InvalidHeader(header) => write!(f, "Invalid header: {header:?}"),
            Self::BufferTooSmall(min_size) => {
                write!(f, "Buffer too small. Expected at least {min_size} bytes.")
            }
            Self::InvalidBufferSize { expected, found } => write!(
                f,
                "Invalid buffer size. Expected {expected} bytes, but found {found} bytes."
            ),
            Self::NoData => write!(f, "No data received."),
            Self::TooMuchData(size) => write!(f, "Too much data: {size} bytes"),
            Self::TooFewData(size) => write!(f, "Too few data: {size} bytes"),
            Self::CannotFindViableChunkSize(size) => {
                write!(f, "Cannot find viable chunk size for {size} bytes")
            }
            Self::Io(error) => write!(f, "{error}"),
            Self::Terminated => write!(f, "terminated"),
        }
    }
}

impl std::error::Error for Error {}

impl From<Error> for std::io::Error {
    fn from(error: Error) -> Self {
        match error {
            Error::InvalidHeader(_)
            | Error::MissingHeader
            | Error::NoData
            | Error::TooMuchData(_)
            | Error::TooFewData(_)
            | Error::CannotFindViableChunkSize(_)
            | Error::Terminated => Self::new(std::io::ErrorKind::InvalidData, error),
            Error::BufferTooSmall(_) | Error::InvalidBufferSize { .. } => {
                Self::new(std::io::ErrorKind::Other, error)
            }
            Error::Io(error) => error,
        }
    }
}

impl From<std::io::Error> for Error {
    fn from(error: std::io::Error) -> Self {
        Self::Io(error)
    }
}
