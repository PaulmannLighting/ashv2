use std::fmt::{Display, Formatter};

#[derive(Clone, Debug)]
#[non_exhaustive]
pub enum Error {
    InvalidHeader(Option<u8>),
    BufferTooSmall { expected: usize, found: usize },
    InvalidBufferSize { expected: usize, found: usize },
    PayloadTooLarge { max: usize, size: usize },
}

impl std::error::Error for Error {}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidHeader(header) => match header {
                Some(id) => write!(f, "Invalid header ID: {id}"),
                None => write!(f, "No header received."),
            },
            Self::BufferTooSmall { expected, found } => {
                write!(
                    f,
                    "Buffer too small. Expected at least {expected} bytes but found {found} bytes."
                )
            }
            Self::InvalidBufferSize { expected, found } => write!(
                f,
                "Invalid buffer size. Expected {expected} bytes, but found {found} bytes."
            ),
            Self::PayloadTooLarge { max, size } => write!(f, "Payload too large: {size} > {max}"),
        }
    }
}
