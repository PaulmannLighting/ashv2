use std::future::Future;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;

use tokio::io::{AsyncRead, AsyncWrite};
use tokio::sync::mpsc::{Sender, channel};

pub use self::futures::Futures;
pub use self::handle::Handle;
pub use self::receiver::Receiver;
pub use self::transmitter::Transmitter;
use crate::types::Payload;

mod futures;
mod handle;
mod message;
mod receiver;
mod transmitter;

/// Create the `ASHv2` actor futures for the given serial port.
///
/// The response channel receives inbound `DATA` payloads from the NCP. Its capacity is also
/// used for the actor's internal message queue.
///
/// Returns the user-facing [`Handle`] and named [`Futures`] that the caller must spawn or
/// otherwise poll on their async runtime.
pub fn start<R, W>(
    reader: R,
    writer: W,
    response: Sender<Payload>,
) -> (
    Handle,
    Futures<impl Future<Output = ()> + Send + 'static, impl Future<Output = ()> + Send + 'static>,
)
where
    R: AsyncRead + Send + Sync + Unpin + 'static,
    W: AsyncWrite + Send + Sync + Unpin + 'static,
{
    let (sender, inbox) = channel(response.capacity());
    let running = Arc::new(AtomicBool::new(true));
    let receiver = Receiver::new(reader, response, sender.clone()).run(running.clone());
    let transmitter = Transmitter::new(writer, inbox, sender.downgrade()).run(running);
    let futures = Futures {
        transmitter,
        receiver,
    };

    (sender.into(), futures)
}
