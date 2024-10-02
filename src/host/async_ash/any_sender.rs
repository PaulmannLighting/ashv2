use std::sync::mpsc::{Sender, SyncSender};

#[derive(Clone, Debug)]
pub enum AnySender<T> {
    Sender(Sender<T>),
    SyncSender(SyncSender<T>),
}

impl<T> AnySender<T> {
    pub fn send(&self, t: T) -> Result<(), std::sync::mpsc::SendError<T>> {
        match self {
            Self::Sender(sender) => sender.send(t),
            Self::SyncSender(sender) => sender.send(t),
        }
    }
}

impl<T> From<Sender<T>> for AnySender<T> {
    fn from(sender: Sender<T>) -> Self {
        Self::Sender(sender)
    }
}

impl<T> From<SyncSender<T>> for AnySender<T> {
    fn from(sender: SyncSender<T>) -> Self {
        Self::SyncSender(sender)
    }
}
