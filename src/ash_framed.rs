use crate::packet::Data;
use crate::response::Response;
use crate::shared_state::SharedState;
use crate::types::FrameBuffer;
use crate::write_frame::WriteFrame;
use crate::{Payload, Request};
use futures::pin_mut;
use serialport::SerialPort;
use std::io::{Error, ErrorKind};
use std::pin::{pin, Pin};
use std::sync::{Arc, Mutex, RwLock, RwLockReadGuard, RwLockWriteGuard};
use std::task::{Context, Poll, Waker};
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};
use tokio::sync::mpsc::{channel, Receiver, Sender};

/// A framed asynchronous `ASHv2` host.
#[derive(Debug)]
pub struct AshFramed<T>
where
    T: SerialPort,
{
    serial_port: T,
    channel_size: usize,
    state: Arc<RwLock<SharedState>>,
    receiver: Option<Receiver<Response>>,
    responses: Arc<Mutex<Option<Sender<Response>>>>,
    frame_buffer: FrameBuffer,
}

impl<T> AshFramed<T>
where
    T: SerialPort,
{
    /// Create a new `AshFramed` instance.
    #[must_use]
    pub fn new(serial_port: T, channel_size: usize) -> Self {
        let state = Arc::new(RwLock::new(SharedState::new()));
        Self {
            serial_port,
            channel_size,
            state,
            receiver: None,
            responses: Arc::new(Mutex::new(None)),
            frame_buffer: FrameBuffer::new(),
        }
    }

    fn state(&self) -> RwLockReadGuard<'_, SharedState> {
        self.state.read().expect("RW lock poisoned.")
    }

    fn state_mut(&self) -> RwLockWriteGuard<'_, SharedState> {
        self.state.write().expect("RW lock poisoned.")
    }

    fn next_data_frame(&mut self, buf: &[u8]) -> std::io::Result<Data> {
        let frame_number = self.state_mut().next_frame_number();
        let ack_number = self.state().ack_number();
        let mut buffer = Payload::new();
        buffer
            .extend_from_slice(buf)
            .map_err(|_| Error::new(ErrorKind::OutOfMemory, "Buffer overflow"))?;
        let data = Data::new(frame_number, buffer, ack_number);
        Ok(data)
    }

    fn reset(&mut self) {
        self.receiver = None;
        self.frame_buffer.clear();
    }
}

impl<T> AsyncWrite for AshFramed<T>
where
    T: SerialPort,
{
    fn poll_write(
        mut self: Pin<&mut AshFramed<T>>,
        _cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<std::io::Result<usize>> {
        let (response_tx, response_rx) = channel(self.channel_size);
        self.responses.lock().unwrap().replace(response_tx);
        pin_mut!(self).receiver.replace(response_rx);
        let data = self.next_data_frame(buf)?;
        self.serial_port
            .write_frame(&data, &mut self.frame_buffer)?;
        Poll::Ready(Ok(buf.len()))
    }

    fn poll_flush(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        Poll::Ready(Ok(()))
    }

    fn poll_shutdown(mut self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        self.reset();
        Poll::Ready(Ok(()))
    }
}

impl<T> AsyncRead for AshFramed<T>
where
    T: SerialPort,
{
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<std::io::Result<()>> {
        let Some(receiver) = &mut self.receiver else {
            return self.reschedule(cx.waker().clone());
        };

        if let Poll::Ready(data) = receiver.recv().poll(cx) {
            if let Some(data) = data {
                buf.put_slice(&data);
                Poll::Ready(Ok(()))
            } else {
                Poll::Ready(Ok(()))
            }
        } else {
            Poll::Pending
        }
    }
}
