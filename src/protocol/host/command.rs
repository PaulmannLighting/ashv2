mod reset_response;
mod response;

pub use reset_response::ResetResponse;
pub use response::{Event, HandleResult, Handler, Response};
use std::sync::Arc;

#[derive(Clone, Debug)]
pub enum Command {
    Data(Arc<[u8]>, Arc<dyn Handler<Arc<[u8]>>>),
    Reset(ResetResponse),
}

impl Command {
    pub fn new<T>(payload: &[u8], handler: T) -> Self
    where
        for<'handler> T: Handler<Arc<[u8]>> + 'handler,
    {
        Self::Data(Arc::from(payload), Arc::new(handler))
    }
}
