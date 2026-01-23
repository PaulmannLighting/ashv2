use log::{debug, error, info, trace, warn};
use serialport::SerialPort;
use tokio::sync::mpsc::Sender;
use tokio::sync::mpsc::error::SendError;

use self::buffer::Buffer;
use crate::actor::message::Message;
use crate::frame::{Ack, Data, Error, Frame, Nak, Rst, RstAck};
use crate::protocol::Mask;
use crate::types::{MAX_FRAME_SIZE, Payload};
use crate::utils::WrappingU3;
use crate::validate::Validate;

mod buffer;

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
    T: SerialPort,
{
    /// Creates a new `ASHv2` receiver.
    pub fn new(serial_port: T, response: Sender<Payload>, transmitter: Sender<Message>) -> Self {
        Self {
            buffer: serial_port.into(),
            response,
            transmitter,
            last_received_frame_num: None,
        }
    }
}

impl<T> Receiver<T>
where
    T: SerialPort + Sync,
{
    /// Runs the receiver loop.
    pub async fn run(mut self) {
        trace!("Starting receiver with frame size: {MAX_FRAME_SIZE}");

        loop {
            let maybe_frame = match self.buffer.read_frame().await {
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

    async fn handle_frame(&mut self, frame: Frame) -> Result<(), SendError<Message>> {
        match frame {
            Frame::Ack(ack) => self.handle_ack(ack).await,
            Frame::Data(data) => self.handle_data(*data).await,
            Frame::Error(error) => self.handle_error(error).await,
            Frame::Nak(nak) => self.handle_nak(nak).await,
            Frame::Rst(rst) => self.handle_rst(rst).await,
            Frame::RstAck(rst_ack) => self.handle_rst_ack(rst_ack).await,
        }
    }

    /// Handle an incoming `ACK` frame.
    async fn handle_ack(&self, ack: Ack) -> Result<(), SendError<Message>> {
        if let Ok(ack) = ack.validate() {
            self.ack_sent_frames(ack.ack_num()).await
        } else {
            warn!("Received ACK with invalid CRC.");
            Ok(())
        }
    }

    /// Handle an incoming `DATA` frame.
    async fn handle_data(&mut self, data: Data) -> Result<(), SendError<Message>> {
        trace!("Handling data frame: {data:#04X}");

        let Ok(data) = data.validate() else {
            warn!("Received data frame with invalid CRC.");
            self.send_nak().await?;
            return Ok(());
        };

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
        if let Ok(error) = error.validate() {
            self.transmitter.send(Message::Error(error)).await
        } else {
            warn!("Received ERROR with invalid CRC.");
            Ok(())
        }
    }

    /// Handle an incoming `NAK` frame.
    async fn handle_nak(&self, nak: Nak) -> Result<(), SendError<Message>> {
        if let Ok(nak) = nak.validate() {
            self.nak_sent_frames(nak.ack_num()).await
        } else {
            warn!("Received NAK with invalid CRC.");
            Ok(())
        }
    }

    async fn handle_rst(&self, rst: Rst) -> Result<(), SendError<Message>> {
        if let Ok(rst) = rst.validate() {
            self.transmitter.send(Message::Rst(rst)).await
        } else {
            warn!("Received RST with invalid CRC.");
            Ok(())
        }
    }

    async fn handle_rst_ack(&self, rst_ack: RstAck) -> Result<(), SendError<Message>> {
        if let Ok(rst_ack) = rst_ack.validate() {
            self.transmitter.send(Message::RstAck(rst_ack)).await
        } else {
            warn!("Received RST-ACK with invalid CRC.");
            Ok(())
        }
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
