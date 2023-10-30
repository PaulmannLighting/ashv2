mod inner;

use crate::Error;
use inner::Inner;
use std::sync::Arc;

pub type BytesIO = Inner<Arc<[u8]>, Arc<[u8]>>;
pub type Reset = Inner<(), ()>;

#[derive(Debug)]
pub enum Transaction {
    Data(BytesIO),
    Reset(Reset),
    Terminate,
}

impl Transaction {
    pub fn new_data(request: &[u8]) -> (Self, BytesIO) {
        let inner = Inner::new(request.into());
        (Self::Data(inner.clone()), inner)
    }

    pub fn new_reset() -> (Self, Reset) {
        let inner = Inner::new(());
        (Self::Reset(inner.clone()), inner)
    }

    pub const fn new_terminate() -> Self {
        Self::Terminate
    }

    pub fn resolve_error(self, error: Error) {
        match self {
            Self::Data(future) => future.resolve(Err(error)),
            Self::Reset(future) => future.resolve(Err(error)),
            Self::Terminate => (),
        }
    }
}
