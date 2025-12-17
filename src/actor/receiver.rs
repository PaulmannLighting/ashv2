use std::io;
use std::io::{BufRead, BufReader, Error, ErrorKind, Read};

use log::{debug, error, trace};
use tokio::sync::mpsc::Sender;

use crate::actor::message::Message;
use crate::frame::Frame;
use crate::protocol::{ControlByte, Stuffing};
use crate::types::RawFrame;
use crate::{HexSlice, Payload};

pub struct Receiver<T> {
    serial_port: BufReader<T>,
    response: Sender<Payload>,
    transmitter: Sender<Message>,
    buffer: RawFrame,
    xon: bool,
}

impl<T> Receiver<T>
where
    T: Read,
{
    pub fn new(serial_port: T, response: Sender<Payload>, transmitter: Sender<Message>) -> Self {
        Self {
            serial_port: BufReader::new(serial_port),
            response,
            transmitter,
            buffer: RawFrame::new(),
            xon: true,
        }
    }

    pub async fn run(&mut self) {
        loop {
            if let Some(frame) = self.receive_frame() {
                self.handle_frame(frame).await;
            }
        }
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
                        } else if self.buffer.push(control_byte.into()).is_err() {
                            trace!("Buffer was: {:#04X}", HexSlice::new(&self.buffer));
                            return Err(Error::other(format!(
                                "Frame buffer overflow: {:#04X}",
                                u8::from(control_byte)
                            )));
                        }
                    }
                },
                Err(byte) => {
                    if self.buffer.push(byte).is_err() {
                        trace!("Buffer was: {:#04X}", HexSlice::new(&self.buffer));
                        return Err(Error::other(format!("Frame buffer overflow: {byte:#04X}")));
                    }
                }
            }
        }

        trace!("Buffer was: {:#04X}", HexSlice::new(&self.buffer));
        Err(Error::new(
            ErrorKind::UnexpectedEof,
            "Byte stream terminated unexpectedly.",
        ))
    }

    async fn handle_frame(&mut self, frame: Frame) {
        match frame {
            Frame::Ack(ack) => {}
            Frame::Data(data) => {}
            Frame::Error(error) => {}
            Frame::Nak(nak) => {}
            Frame::Rst(rst) => {}
            Frame::RstAck(rst_ack) => {}
        }
    }

    /// Handle an incoming `DATA` frame.
    fn handle_data(&mut self, data: Data) -> io::Result<()> {
        trace!("Handling data frame: {data:#04X}");

        if !data.is_crc_valid() {
            warn!("Received data frame with invalid CRC.");
            self.enter_reject()?;
        } else if data.frame_num() == self.state.ack_number() {
            self.leave_reject();
            self.state.set_last_received_frame_num(data.frame_num());
            self.send_ack()?;
            self.ack_sent_frames(data.ack_num());
            self.handle_payload(data.into_payload());
        } else if data.is_retransmission() {
            info!("Received retransmission of frame: {data}");
            self.ack()?;
            self.ack_sent_frames(data.ack_num());
            self.handle_payload(data.into_payload());
        } else {
            warn!("Received out-of-sequence data frame: {data}");
            self.enter_reject()?;
        }

        Ok(())
    }

    /// Handle an incoming `NAK` frame.
    fn handle_nak(&mut self, nak: &Nak) -> io::Result<()> {
        if !nak.is_crc_valid() {
            warn!("Received ACK with invalid CRC.");
        }

        self.nak_sent_frames(nak.ack_num())
    }

    /// Handle an incoming `ACK` frame.
    fn handle_ack(&mut self, ack: &Ack) {
        if !ack.is_crc_valid() {
            warn!("Received ACK with invalid CRC.");
        }

        self.ack_sent_frames(ack.ack_num());
    }
}
