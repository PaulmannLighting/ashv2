use std::io;
use std::io::{ErrorKind, Read};
use std::time::Duration;

use log::{debug, error, info, trace, warn};
use tokio::sync::mpsc::Sender;
use tokio::sync::mpsc::error::SendError;
use tokio::time::sleep;

use self::buffer::Buffer;
use crate::actor::message::Message;
use crate::frame::{Ack, Data, Error, Frame, Nak, Rst, RstAck};
use crate::protocol::Mask;
use crate::types::Payload;
use crate::utils::WrappingU3;
use crate::validate::Validate;

mod buffer;

const BURST: Duration = Duration::from_millis(100);

/// `ASHv2` receiver.
#[derive(Debug)]
pub struct Receiver<T> {
    buffer: Buffer<T>,
    response: Sender<Payload>,
    transmitter: Sender<Message>,
    last_received_frame_num: Option<WrappingU3>,
}

impl<T> Receiver<T>
where
    T: Read + Sync,
{
    /// Creates a new `ASHv2` receiver.
    pub const fn new(
        serial_port: T,
        response: Sender<Payload>,
        transmitter: Sender<Message>,
    ) -> Self {
        Self {
            buffer: Buffer::new(serial_port),
            response,
            transmitter,
            last_received_frame_num: None,
        }
    }

    /// Runs the receiver loop.
    pub async fn run(mut self) {
        trace!("Starting receiver");

        loop {
            let maybe_frame = match self.receive_frame() {
                Ok(maybe_frame) => maybe_frame,
                Err(error) => {
                    error!("Error receiving frame: {error}");
                    continue;
                }
            };

            if let Some(frame) = maybe_frame {
                trace!("Received frame: {frame:#04X}");

                if let Err(error) = self.handle_frame(frame).await {
                    info!("Transmitter channel closed, receiver exiting: {error}");
                    break;
                }
            } else {
                // Prevent blocking of main thread in async environment.
                sleep(BURST).await;
            }
        }
    }

    /// Returns the ACK number.
    ///
    /// This is equal to the last received frame number plus one.
    fn ack_number(&self) -> WrappingU3 {
        self.last_received_frame_num
            .map_or_else(WrappingU3::default, |ack_number| ack_number + 1u8)
    }

    fn receive_frame(&mut self) -> io::Result<Option<Frame>> {
        match self.buffer.read_frame() {
            Ok(frame) => Ok(Some(frame)),
            Err(error) => {
                if error.kind() != ErrorKind::TimedOut {
                    return Err(error);
                }

                Ok(None)
            }
        }
    }

    async fn handle_frame(&mut self, frame: Frame) -> Result<(), SendError<Message>> {
        match frame {
            Frame::Ack(ack) => self.handle_ack(&ack).await,
            Frame::Data(data) => self.handle_data(*data).await,
            Frame::Error(error) => self.handle_error(error).await,
            Frame::Nak(nak) => self.handle_nak(&nak).await,
            Frame::Rst(rst) => self.handle_rst(rst).await,
            Frame::RstAck(rst_ack) => self.handle_rst_ack(rst_ack).await,
        }
    }

    /// Handle an incoming `ACK` frame.
    async fn handle_ack(&self, ack: &Ack) -> Result<(), SendError<Message>> {
        if !ack.is_crc_valid() {
            warn!("Received ACK with invalid CRC.");
        }

        self.ack_sent_frames(ack.ack_num()).await
    }

    /// Handle an incoming `DATA` frame.
    async fn handle_data(&mut self, data: Data) -> Result<(), SendError<Message>> {
        trace!("Handling data frame: {data:#04X}");

        if !data.is_crc_valid() {
            warn!("Received data frame with invalid CRC.");
            self.send_nak().await?;
            return Ok(());
        }

        if data.frame_num() == self.ack_number() {
            trace!("Received in-sequence data frame: {data}");
            self.last_received_frame_num.replace(data.frame_num());
            self.send_ack().await?;
            self.ack_sent_frames(data.ack_num()).await?;
            self.handle_payload(data.into_payload()).await;
            return Ok(());
        }

        if data.is_retransmission() {
            debug!("Received retransmission of data frame: {data}");
            self.send_ack().await?;
            self.ack_sent_frames(data.ack_num()).await?;
            self.handle_payload(data.into_payload()).await;
            return Ok(());
        }

        warn!("Received out-of-sequence data frame: {data}");
        self.send_nak().await?;
        Ok(())
    }

    async fn handle_error(&self, error: Error) -> Result<(), SendError<Message>> {
        if !error.is_crc_valid() {
            warn!("Received ERROR with invalid CRC.");
        }

        self.transmitter.send(Message::Error(error)).await
    }

    /// Handle an incoming `NAK` frame.
    async fn handle_nak(&self, nak: &Nak) -> Result<(), SendError<Message>> {
        if !nak.is_crc_valid() {
            warn!("Received NAK with invalid CRC.");
        }

        self.nak_sent_frames(nak.ack_num()).await
    }

    async fn handle_rst(&self, rst: Rst) -> Result<(), SendError<Message>> {
        if !rst.is_crc_valid() {
            warn!("Received RST with invalid CRC.");
        }

        self.transmitter.send(Message::Rst(rst)).await
    }

    async fn handle_rst_ack(&self, rst_ack: RstAck) -> Result<(), SendError<Message>> {
        if !rst_ack.is_crc_valid() {
            warn!("Received RST-ACK with invalid CRC.");
        }

        self.transmitter.send(Message::RstAck(rst_ack)).await
    }

    /// Send the response frame's payload through the response channel.
    async fn handle_payload(&self, mut payload: Payload) {
        payload.mask();
        self.response.send(payload).await.unwrap_or_else(|error| {
            error!("Failed to send payload through response channel: {error}");
        });
    }

    /// Send an `ACK` frame.
    async fn send_ack(&self) -> Result<(), SendError<Message>> {
        self.transmitter.send(Message::Ack(self.ack_number())).await
    }

    /// Send a `NAK` frame.
    async fn send_nak(&self) -> Result<(), SendError<Message>> {
        self.transmitter.send(Message::Nak(self.ack_number())).await
    }

    /// Acknowledge sent frames up to `ack_num`.
    async fn ack_sent_frames(&self, ack_num: WrappingU3) -> Result<(), SendError<Message>> {
        self.transmitter.send(Message::AckSentFrame(ack_num)).await
    }

    /// Negative acknowledge sent frames up to `ack_num`.
    async fn nak_sent_frames(&self, ack_num: WrappingU3) -> Result<(), SendError<Message>> {
        self.transmitter.send(Message::NakSentFrame(ack_num)).await
    }
}
