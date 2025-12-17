use tokio::sync::mpsc::{Receiver, Sender};

use crate::actor::message::Message;

pub struct Transmitter<T> {
    serial_port: T,
    messages: Receiver<Message>,
    requeue: Sender<Message>,
}

impl<T> Transmitter<T> {
    pub const fn new(
        serial_port: T,
        messages: Receiver<Message>,
        requeue: Sender<Message>,
    ) -> Self {
        Self {
            serial_port,
            messages,
            requeue,
        }
    }
}
