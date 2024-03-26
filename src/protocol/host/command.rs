use std::sync::Arc;

pub use reset_response::ResetResponse;
pub use response::{Event, HandleResult, Handler, Response};

mod reset_response;
mod response;

#[derive(Clone, Debug)]
pub enum Command<'handler> {
    Data(Arc<[u8]>, Arc<dyn Handler<Arc<[u8]>> + 'handler>),
    Reset(ResetResponse),
}

impl<'handler> Command<'handler> {
    pub fn new(payload: &[u8], handler: Arc<dyn Handler<Arc<[u8]>> + 'handler>) -> Self {
        Self::Data(Arc::from(payload), handler)
    }
}
