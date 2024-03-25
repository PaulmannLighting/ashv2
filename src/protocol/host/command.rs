mod reset_response;
mod response;

pub use reset_response::ResetResponse;
pub use response::{Event, HandleResult, Handler, Response};
use std::sync::Arc;

#[derive(Clone, Debug)]
pub enum Command {
    Data(Arc<[u8]>, Arc<dyn Handler<Arc<[u8]>> + 'static>),
    Reset(ResetResponse),
}

impl Command {
    pub fn new<T>(request: &[u8], response: T) -> Self
    where
        T: Handler<Arc<[u8]>> + 'static,
    {
        Self::Data(Arc::from(request), Arc::new(response))
    }
}
