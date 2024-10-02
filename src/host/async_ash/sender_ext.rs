use std::sync::mpsc::{SendError, Sender, SyncSender};

pub trait SenderExt<T> {
    fn send(&self, t: T) -> Result<(), SendError<T>>;
}

impl<T> SenderExt<T> for Sender<T> {
    fn send(&self, t: T) -> Result<(), SendError<T>> {
        self.send(t)
    }
}

impl<T> SenderExt<T> for SyncSender<T> {
    fn send(&self, t: T) -> Result<(), SendError<T>> {
        self.send(t)
    }
}
