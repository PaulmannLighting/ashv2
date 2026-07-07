use log::debug;
use serialport::SerialPort;
use tokio::spawn;
use tokio::sync::mpsc::{Receiver, channel};

use self::message::Message;
use self::reader::Reader;
use self::writer::Writer;

mod message;
mod reader;
mod writer;

#[derive(Debug)]
pub struct AsyncSerialPort<T>(T);

impl<T> AsyncSerialPort<T> {
    pub const fn new(port: T) -> Self {
        Self(port)
    }
}

impl<T> AsyncSerialPort<T>
where
    T: SerialPort,
{
    async fn run(mut self, mut inbox: Receiver<Message>) {
        while let Some(msg) = inbox.recv().await {
            match msg {
                Message::Write { buffer, response } => {
                    match self.0.write_all(&buffer) {
                        Ok(()) => response.send(Ok(())),
                        Err(error) => response.send(Err(error)),
                    }
                    .unwrap_or_else(|error| debug!("Failed to send read response: {error:?}"));
                }
                Message::Read {
                    mut buffer,
                    response,
                } => response
                    .send(self.0.read(&mut buffer).map(|_| buffer))
                    .unwrap_or_else(|error| debug!("Failed to send write response: {error:?}")),
                Message::Flush(response) => response
                    .send(self.0.flush())
                    .unwrap_or_else(|error| debug!("Failed to send flush response: {error:?}")),
            }
        }
    }
}

impl<T> AsyncSerialPort<T>
where
    T: SerialPort + 'static,
{
    pub fn spawn(self, channel_size: usize) -> (Reader, Writer) {
        let (tx, rx) = channel(channel_size);
        spawn(self.run(rx));
        (Reader(tx.clone()), Writer(tx))
    }
}
