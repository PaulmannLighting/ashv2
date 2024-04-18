use std::sync::Arc;

use super::response::Handler;

#[derive(Clone, Debug)]
pub struct Command {
    pub(crate) payload: Arc<[u8]>,
    pub(crate) handler: Arc<dyn Handler>,
}

impl Command {
    #[must_use]
    pub const fn new(payload: Arc<[u8]>, handler: Arc<dyn Handler>) -> Self {
        Self { payload, handler }
    }
}
