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

/// Create the `ASHv2` actor futures for the given asynchronous reader and writer.
///
/// The response channel receives inbound `DATA` payloads from the NCP. Its capacity is also
/// used for the actor's internal message queue.
///
/// The caller is responsible for opening and configuring the underlying transport and splitting
/// it into reader and writer values when necessary. Both transport values must be `Send + Unpin`
/// and may contain borrows, including mutable references such as `&mut reader` and `&mut writer`.
///
/// Returns the user-facing [`Handle`] and two `Send` actor futures in [`Futures`]. Poll both
/// concurrently. With borrowed I/O, poll the futures within the scope of the borrows.
/// Each opaque future type captures both `R` and `W`, so both transport types must be `'static`
/// to pass either future to `tokio::spawn`.
///
/// Drop every clone of the handle to close the outbound queue and terminate the transmitter
/// after queued messages have been drained. The transmitter then signals the receiver to
/// terminate.
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

#[cfg(test)]
mod tests {
    use tokio::io::{empty, sink};

    use super::*;

    const RESPONSE_CAPACITY: usize = 1;

    #[test]
    fn start_accepts_send_only_io() {
        fn assert_send<T>(_: T)
        where
            T: Send,
        {
        }

        let reader: Box<dyn AsyncRead + Send + Unpin> = Box::new(empty());
        let writer: Box<dyn AsyncWrite + Send + Unpin> = Box::new(sink());
        let (response, _responses) = channel(RESPONSE_CAPACITY);

        let (_handle, futures) = start(reader, writer, response);
        assert_send(futures);
    }
}
