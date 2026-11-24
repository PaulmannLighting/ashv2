use std::future::Future;

use tokio::io::{AsyncRead, AsyncWrite};
use tokio::sync::mpsc::{Sender, channel};

pub use self::futures::Futures;
pub use self::handle::Handle;
pub use self::receiver::Receiver;
pub use self::transmitter::Transmitter;
use crate::TX_K;
use crate::types::Payload;

mod futures;
mod handle;
mod message;
mod receiver;
#[cfg(test)]
mod tests;
mod timer;
mod transmitter;

/// Create the `ASHv2` actor futures for the given asynchronous reader and writer.
///
/// The response channel receives inbound `DATA` payloads from the NCP. Its capacity is also
/// used for the outbound request queue.
///
/// The caller is responsible for opening and configuring the underlying transport and splitting
/// it into reader and writer values when necessary. Both transport values must be `Send + Unpin`
/// and may contain borrows, including mutable references such as `&mut reader` and `&mut writer`.
///
/// Returns the user-facing [`Handle`] and two `Send` actor futures in [`Futures`]. Poll both
/// concurrently on the executor of your choice. With borrowed I/O, poll the futures within the
/// scope of the borrows.
/// Each opaque future type captures both `R` and `W`, so both transport types must be `'static`
/// to pass either future to `tokio::spawn`.
///
/// Drop every clone of the handle to close the outbound queue. Both futures terminate after
/// queued writes are drained or the link fails. On link failure, start a fresh ASH session
/// and renegotiate EZSP before sending more commands.
pub fn start<R, W>(
    reader: R,
    writer: W,
    response: Sender<Payload>,
) -> (
    Handle,
    Futures<impl Future<Output = ()> + Send, impl Future<Output = ()> + Send>,
)
where
    R: AsyncRead + Send + Unpin,
    W: AsyncWrite + Send + Unpin,
{
    let (sender, inbox) = channel(response.max_capacity());
    let (frames, incoming) = channel(TX_K);
    let receiver = Receiver::new(reader, frames).run();
    let transmitter = Transmitter::new(writer, inbox, incoming, response).run();
    let futures = Futures {
        transmitter,
        receiver,
    };

    (sender.into(), futures)
}
