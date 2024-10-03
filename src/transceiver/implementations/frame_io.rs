//! `ASHv2` frame I/O implementation.
//!
//! This module contains the implementation of the `ASHv2` frame I/O operations.
use crate::frame::Frame;
use crate::packet::{Ack, Data, Nak, Packet, RST};
use crate::protocol::{Stuffing, CANCEL, FLAG, SUBSTITUTE, WAKE, X_OFF, X_ON};
use crate::transceiver::Transceiver;
use log::{debug, trace, warn};
use serialport::SerialPort;
use std::io::{Error, ErrorKind, Read};
use std::time::SystemTime;

impl<T> Transceiver<T>
where
    T: SerialPort,
{
    /// Receives a packet from the serial port.
    ///
    /// Returns `Ok(None)` if no packet was received within the timeout.
    ///
    /// # Errors
    ///
    /// Returns an [Error] if the serial port read operation failed.
    pub(in crate::transceiver) fn receive(&mut self) -> std::io::Result<Option<Packet>> {
        match self.read_packet() {
            Ok(packet) => Ok(Some(packet)),
            Err(error) => {
                if error.kind() == ErrorKind::TimedOut {
                    Ok(None)
                } else {
                    Err(error)
                }
            }
        }
    }

    /// Send an `ACK` frame with the given ACK number.
    ///
    /// # Errors
    ///
    /// Returns an [Error] if the serial port read operation failed.
    pub(in crate::transceiver) fn ack(&mut self) -> std::io::Result<()> {
        self.send_ack(&Ack::new(self.state.ack_number(), self.state.n_rdy()))
    }

    /// Send a `NAK` frame with the current ACK number.
    ///
    /// # Errors
    ///
    /// Returns an [Error] if the serial port read operation failed.
    pub(in crate::transceiver) fn nak(&mut self) -> std::io::Result<()> {
        self.send_nak(&Nak::new(self.state.ack_number(), self.state.n_rdy()))
    }

    /// Send a RST frame.
    ///
    /// # Errors
    ///
    /// Returns an [Error] if the serial port read operation failed.
    pub(in crate::transceiver) fn rst(&mut self) -> std::io::Result<()> {
        self.write_frame(&RST)
    }

    /// Send a `DATA` frame.
    ///
    /// # Errors
    ///
    /// Returns an [Error] if the serial port read operation failed.
    pub(in crate::transceiver) fn send_data(&mut self, data: Data) -> std::io::Result<()> {
        self.write_frame(&data)?;
        self.enqueue_sent_data(data)
    }

    /// Retransmit a `DATA` frame.
    ///
    /// Set the `is_retransmission` flag in the `DATA` frame and send it.
    ///
    /// # Errors
    ///
    /// Returns an [Error] if the serial port read operation failed.
    pub(in crate::transceiver) fn retransmit(&mut self, mut data: Data) -> std::io::Result<()> {
        data.set_is_retransmission(true);
        self.send_data(data)
    }

    /// Send a raw `ACK` frame.
    fn send_ack(&mut self, ack: &Ack) -> std::io::Result<()> {
        if ack.not_ready() {
            self.state
                .last_n_rdy_transmission
                .replace(SystemTime::now());
        }

        debug!("Sending ACK: {ack}");
        self.write_frame(ack)
    }

    /// Send a raw `NAK` frame.
    fn send_nak(&mut self, nak: &Nak) -> std::io::Result<()> {
        if nak.not_ready() {
            self.state
                .last_n_rdy_transmission
                .replace(SystemTime::now());
        }

        debug!("Sending NAK: {nak}");
        self.write_frame(nak)
    }

    /// Read an ASH [`Packet`].
    ///
    /// # Errors
    ///
    /// Returns an [`Error`] if any I/O, protocol or parsing error occurs.
    fn read_packet(&mut self) -> std::io::Result<Packet> {
        self.buffer_frame()?;
        Packet::try_from(self.buffers.frame.as_slice())
    }

    /// Reads an ASH frame into the transceiver's frame buffer.
    ///
    /// # Errors
    /// Returns an [`Error`] if any I/O or protocol error occurs.
    fn buffer_frame(&mut self) -> std::io::Result<()> {
        let buffer = &mut self.buffers.frame;
        buffer.clear();
        let serial_port = &mut self.serial_port;
        let mut error = false;

        for byte in serial_port.bytes() {
            match byte? {
                CANCEL => {
                    trace!("Resetting buffer due to cancel byte.");
                    buffer.clear();
                    error = false;
                }
                FLAG => {
                    trace!("Received flag byte.");

                    if !error && !buffer.is_empty() {
                        debug!("Received frame.");
                        trace!("Buffer: {buffer:#04X?}");
                        buffer.unstuff();
                        trace!("Unstuffed buffer: {:#04X?}", buffer);
                        return Ok(());
                    }

                    trace!("Resetting buffer due to error or empty buffer.");
                    trace!("Error condition was: {error}");
                    trace!("Buffer: {buffer:#04X?}");
                    buffer.clear();
                    error = false;
                }
                SUBSTITUTE => {
                    trace!("Received SUBSTITUTE byte. Setting error condition.");
                    error = true;
                }
                X_ON => {
                    warn!("NCP requested to resume transmission. Ignoring.");
                }
                X_OFF => {
                    warn!("NCP requested to stop transmission. Ignoring.");
                }
                WAKE => {
                    debug!("NCP tried to wake us up.");
                }
                byte => {
                    if buffer.push(byte).is_err() {
                        return Err(Error::new(
                            ErrorKind::OutOfMemory,
                            "ASHv2 frame buffer overflow",
                        ));
                    }
                }
            }
        }

        Err(Error::new(
            ErrorKind::UnexpectedEof,
            "byte stream terminated unexpectedly",
        ))
    }

    /// Writes an ASH [`Frame`].
    ///
    /// # Errors
    ///
    /// Returns an [Error] if the serial port write operation failed.
    fn write_frame<F>(&mut self, frame: &F) -> std::io::Result<()>
    where
        F: Frame,
    {
        let buffer = &mut self.buffers.frame;
        debug!("Writing frame: {frame}");
        trace!("Frame: {frame:#04X?}");
        buffer.clear();
        frame.buffer(buffer).map_err(|()| {
            Error::new(
                ErrorKind::OutOfMemory,
                "could not append frame bytes to buffer",
            )
        })?;
        trace!("Frame bytes: {buffer:#04X?}");
        buffer.stuff()?;
        trace!("Stuffed bytes: {buffer:#04X?}");
        buffer
            .push(FLAG)
            .map_err(|_| Error::new(ErrorKind::OutOfMemory, "could not append flag byte"))?;
        trace!("Writing bytes: {buffer:#04X?}");
        self.serial_port.write_all(buffer)?;
        self.serial_port.flush()
    }
}
