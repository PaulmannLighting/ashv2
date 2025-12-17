use std::io;
use std::io::{ErrorKind, Write};
use std::thread::sleep;
use std::time::{Duration, Instant};

use log::{debug, error, info, trace, warn};
use serialport::SerialPort;
use tokio::sync::mpsc::{Receiver, Sender};
use tokio::sync::oneshot;

use self::buffer::Buffer;
use crate::actor::message::Message;
use crate::frame::{Ack, Error, Frame, Nak, RST, Rst};
use crate::status::Status;
use crate::utils::WrappingU3;

mod buffer;

const T_RSTACK_MAX: Duration = Duration::from_millis(3200);

/// `ASHv2` transmitter.
#[derive(Debug)]
pub struct Transmitter<T> {
    buffer: Buffer<T>,
    messages: Receiver<Message>,
    requeue: Sender<Message>,
    status: Status,
    last_rst_sent: Option<Instant>,
}

impl<T> Transmitter<T> {
    /// Creates a new `ASHv2` transmitter.
    #[must_use]
    pub const fn new(
        serial_port: T,
        messages: Receiver<Message>,
        requeue: Sender<Message>,
    ) -> Self {
        Self {
            buffer: Buffer::new(serial_port),
            messages,
            requeue,
            status: Status::Disconnected,
            last_rst_sent: None,
        }
    }
}

impl<T> Transmitter<T>
where
    T: SerialPort,
{
    pub async fn run(mut self) {
        while let Some(message) = self.messages.recv().await {
            self.handle_message(message).await;
        }

        info!("Message channel closed, transmitter exiting.");
    }

    async fn handle_message(&mut self, message: Message) {
        trace!("Received message to transmit: {message:?}");

        match message {
            Message::Payload { payload, response } => self.transmit(payload, response).await,
            Message::Ack(ack_num) => self.send_ack(ack_num).await,
            Message::Nak(ack_num) => self.send_nak(ack_num).await,
            Message::Rst(rst) => self.handle_rst(rst).await,
            Message::RstAck(rst_ack) => self.handle_rst_ack(rst_ack).await,
            Message::Error(error) => self.handle_error(error).await,
            Message::AckSentFrame(frame_num) => self.ack_sent_frames(frame_num).await,
            Message::NakSentFrame(frame_num) => self.nak_sent_frames(frame_num).await,
        }
    }

    async fn transmit(&mut self, payload: Box<[u8]>, response: oneshot::Sender<io::Result<()>>) {
        if self.status != Status::Connected {
            self.requeue
                .send(Message::Payload { payload, response })
                .await
                .unwrap_or_else(|error| {
                    trace!("Failed to requeue payload message: {error}");
                });

            return self.connect();
        }
        todo!()
    }

    async fn send_ack(&mut self, ack_num: WrappingU3) -> io::Result<()> {
        self.buffer.write_frame(Ack::new(ack_num, false))
    }

    async fn send_nak(&mut self, ack_num: WrappingU3) -> io::Result<()> {
        self.buffer.write_frame(Nak::new(ack_num, false))
    }

    /// Handle RST frame received from the NCP.
    fn handle_rst(&mut self, rst: Rst) -> io::Result<()> {
        error!("Received RST frame: {rst}, resetting connection.");
        self.status = Status::Disconnected;
        self.reset()
    }

    /// Handle RST ACK frame received from the NCP.
    fn handle_rst_ack(&mut self, rst_ack: Rst) -> io::Result<()> {
        trace!("Received RST ACK frame: {rst_ack}, connection reset acknowledged.");

        if let Some(timestamp) = self.last_rst_sent.take() {
            if timestamp.elapsed() < T_RSTACK_MAX {
                debug!("Connection established successfully.");
                self.status = Status::Connected;
                Ok(())
            } else {
                warn!("RST ACK received after timeout. Resetting connection again.");
                self.reset()
            }
        } else {
            warn!("Received unexpected RST ACK frame: {rst_ack}.");
            Ok(())
        }
    }

    /// Handle errors received from the NCP.
    fn handle_error(&mut self, error: Error) -> io::Result<()> {
        warn!("Transmitter encountered error: {error}, resetting connection.");
        self.status = Status::Failed;
        self.reset()
    }

    /// Send RST frame to reset the connection.
    fn reset(&mut self) -> io::Result<()> {
        if let Some(timestamp) = self.last_rst_sent.take() {
            if timestamp.elapsed() < T_RSTACK_MAX {
                debug!("Last RST sent {timestamp:?} ago, waiting before sending another...");
                self.last_rst_sent.replace(timestamp);
                return Ok(());
            }
        }

        self.last_rst_sent.replace(Instant::now());
        self.buffer.write_frame(RST)
    }
}
