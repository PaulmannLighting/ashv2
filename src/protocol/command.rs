use std::sync::Arc;

use super::response::Handler;

#[derive(Clone, Debug)]
pub struct Command<'cmd> {
    pub(crate) payload: Arc<[u8]>,
    pub(crate) handler: Arc<dyn Handler + 'cmd>,
}

impl<'cmd> Command<'cmd> {
    #[must_use]
    pub const fn new(payload: Arc<[u8]>, handler: Arc<dyn Handler + 'cmd>) -> Self {
        Self { payload, handler }
    }
}
