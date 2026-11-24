use log::{debug, warn};
use tokio::io::AsyncRead;
use tokio::select;
use tokio::sync::mpsc::Sender;

use self::buffer::Buffer;
use crate::frame::Frame;

mod buffer;

/// An inbound frame or a framing error to handle in wire order.
#[derive(Debug)]
pub enum Received {
    Frame(Frame),

    Invalid,
}

/// Reads serial frames and forwards them to the actor that owns link state.
#[derive(Debug)]
pub struct Receiver<R> {
    buffer: Buffer<R>,
    frames: Sender<Received>,
}

impl<R> Receiver<R>
where
    R: AsyncRead,
{
    /// Creates a reader for the receive half of the transport.
    pub fn new(reader: R, frames: Sender<Received>) -> Self {
        Self {
            buffer: Buffer::new(reader),
            frames,
        }
    }
}

impl<R> Receiver<R>
where
    R: AsyncRead + Unpin,
{
    /// Forwards frames until the link ends or the transport can no longer be read.
    pub async fn run(mut self) {
        loop {
            let received = select! {
                biased;
                () = self.frames.closed() => break,
                frame = self.buffer.read_frame() => frame,
            };
            match received {
                Ok(frame) => {
                    if self.frames.send(frame).await.is_err() {
                        break;
                    }
                }
                Err(error) => {
                    warn!("ASHv2 receive transport closed: {error}");
                    break;
                }
            }
        }
        debug!("ASHv2 receiver stopped");
    }
}
