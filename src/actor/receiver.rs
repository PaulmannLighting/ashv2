use std::io;
use std::io::{BufReader, ErrorKind, Read};

use log::{debug, error, trace, warn};
use tokio::sync::mpsc::Sender;
use tokio::sync::mpsc::error::SendError;

use crate::actor::message::Message;
use crate::frame::{Ack, Data, Error, Frame, Nak, Rst, RstAck};
use crate::protocol::{ControlByte, Mask, Unstuff};
use crate::utils::WrappingU3;
use crate::validate::Validate;
use crate::{HexSlice, Payload};

/// `ASHv2` receiver.
#[derive(Debug)]
pub struct Receiver<T> {
    serial_port: BufReader<T>,
    response: Sender<Payload>,
    transmitter: Sender<Message>,
    buffer: Vec<u8>,
    xon: bool,
    last_received_frame_num: Option<WrappingU3>,
}

impl<T> Receiver<T>
where
    T: Read,
{
    /// Creates a new `ASHv2` receiver.
    pub fn new(serial_port: T, response: Sender<Payload>, transmitter: Sender<Message>) -> Self {
        Self {
            serial_port: BufReader::new(serial_port),
            response,
            transmitter,
            buffer: Vec::new(),
            xon: true,
            last_received_frame_num: None,
        }
    }
}

impl<T> Receiver<T>
where
    T: Read + Sync,
{
    /// Runs the receiver loop.
    pub async fn run(&mut self) {
        loop {
            if let Some(frame) = self.receive_frame() {
                if let Err(error) = self.handle_frame(frame).await {
                    error!("Error handling received frame: {error}");
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
            .map_or_else(WrappingU3::default, |ack_number| ack_number + 1)
    }

    fn receive_frame(&mut self) -> Option<Frame> {
        match self.read_frame() {
            Ok(frame) => Some(frame),
            Err(error) => {
                if error.kind() != ErrorKind::TimedOut {
                    error!("Error receiving frame: {error}");
                }

                None
            }
        }
    }

    /// Read an `ASHv2` [`Frame`].
    ///
    /// # Errors
    ///
    /// Returns an [`Error`] if any I/O, protocol or parsing error occurs.
    fn read_frame(&mut self) -> io::Result<Frame> {
        self.read_raw_frame()?.try_into()
    }

    /// Reads an `ASHv2` frame into the buffer.
    ///
    /// # Errors
    ///
    /// Returns an [`Error`] if any I/O or protocol error occurs.
    fn read_raw_frame(&mut self) -> io::Result<&[u8]> {
        self.buffer.clear();
        let mut error = false;

        for byte in (&mut self.serial_port).bytes() {
            match ControlByte::try_from(byte?) {
                Ok(control_byte) => match control_byte {
                    ControlByte::Cancel => {
                        trace!("Resetting buffer due to cancel byte.");
                        self.buffer.clear();
                        error = false;
                    }
                    ControlByte::Flag => {
                        trace!("Received flag byte.");

                        if !error && !self.buffer.is_empty() {
                            debug!("Received frame.");
                            trace!("Buffer: {:#04X}", HexSlice::new(&self.buffer));
                            self.buffer.unstuff();
                            trace!("Unstuffed buffer: {:#04X}", HexSlice::new(&self.buffer));
                            return Ok(&self.buffer);
                        }

                        trace!("Resetting buffer due to error or empty buffer.");
                        trace!("Error condition was: {error}");
                        trace!("Buffer: {:#04X}", HexSlice::new(&self.buffer));
                        self.buffer.clear();
                        error = false;
                    }
                    ControlByte::Substitute => {
                        trace!("Received SUBSTITUTE byte. Setting error condition.");
                        error = true;
                    }
                    ControlByte::Xon => {
                        trace!("NCP requested to resume transmission.");
                        self.xon = true;
                    }
                    ControlByte::Xoff => {
                        trace!("NCP requested to stop transmission.");
                        self.xon = false;
                    }
                    ControlByte::Wake => {
                        if self.buffer.is_empty() {
                            debug!("NCP tried to wake us up.");
                        } else {
                            self.buffer.push(control_byte.into());
                        }
                    }
                },
                Err(byte) => {
                    self.buffer.push(byte);
                }
            }
        }

        trace!("Buffer state: {:#04X}", HexSlice::new(&self.buffer));
        Err(io::Error::new(
            ErrorKind::UnexpectedEof,
            "Byte stream terminated unexpectedly.",
        ))
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
            self.send_nak(None).await?;
            return Ok(());
        }

        if data.frame_num() == self.ack_number() {
            trace!("Received in-sequence data frame: {data}");
            self.last_received_frame_num.replace(data.frame_num());
            self.send_ack(data.frame_num()).await?;
            self.ack_sent_frames(data.ack_num()).await?;
            self.handle_payload(data.into_payload()).await;
            return Ok(());
        }

        if data.is_retransmission() {
            debug!("Received retransmission of data frame: {data}");
            self.send_ack(data.frame_num()).await?;
            self.ack_sent_frames(data.ack_num()).await?;
            self.handle_payload(data.into_payload()).await;
            return Ok(());
        }

        warn!("Received out-of-sequence data frame: {data}");
        self.send_nak(None).await?;
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
            error!("Failed to send payload through response channel: {error}")
        });
    }

    /// Send an `ACK` frame.
    async fn send_ack(&self, frame_num: WrappingU3) -> Result<(), SendError<Message>> {
        self.transmitter.send(Message::Ack(frame_num)).await
    }

    /// Send a `NAK` frame.
    async fn send_nak(&self, frame_num: Option<WrappingU3>) -> Result<(), SendError<Message>> {
        self.transmitter.send(Message::Nak(frame_num)).await
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
