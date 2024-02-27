mod reset_response;
mod response;

pub use reset_response::ResetResponse;
pub use response::{Event, HandleResult, Response};
use std::sync::Arc;

#[derive(Clone, Debug)]
pub enum Command {
    Data(Arc<[u8]>, Arc<dyn Response<Arc<[u8]>>>),
    Reset(ResetResponse),
}

impl Command {
    pub fn new<T>(request: &[u8], response: T) -> Self
    where
        T: Response<Arc<[u8]>> + 'static,
    {
        Self::Data(Arc::from(request), Arc::new(response))
    }
}
