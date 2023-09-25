use std::fmt::{Display, Formatter};

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Error {
    MissingHeader,
    InvalidHeader(u8),
    BufferTooSmall(usize),
    InvalidBufferSize { expected: usize, found: usize },
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
        }
    }
}

impl std::error::Error for Error {}
