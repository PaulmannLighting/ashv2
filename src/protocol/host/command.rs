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
