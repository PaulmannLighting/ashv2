use crate::Payload;
use std::sync::mpsc::{sync_channel, Receiver, SyncSender};

/// An iterator over received callback frames.
#[derive(Debug)]
pub struct Callbacks {
    callbacks: Receiver<Payload>,
}

impl Callbacks {
    /// Create a new `Callbacks` instance.
    #[must_use]
    pub const fn new(callbacks: Receiver<Payload>) -> Self {
        Self { callbacks }
    }

    /// Create a new `Callbacks` instance with a channel and return the sender and itself.
    #[must_use]
    pub fn create(channel_buffer: usize) -> (SyncSender<Payload>, Self) {
        let (sender, receiver) = sync_channel(channel_buffer);
        (sender, Self::new(receiver))
    }
}

impl From<Receiver<Payload>> for Callbacks {
    fn from(callbacks: Receiver<Payload>) -> Self {
        Self::new(callbacks)
    }
}

impl Iterator for Callbacks {
    type Item = Payload;

    fn next(&mut self) -> Option<Self::Item> {
        self.callbacks.recv().ok()
    }
}
