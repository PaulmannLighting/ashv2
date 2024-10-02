use crate::frame::Frame;
use crate::packet::Packet;
use crate::protocol::{Stuffing, CANCEL, FLAG, SUBSTITUTE, WAKE, X_OFF, X_ON};
use crate::transceiver::Transceiver;
use log::{debug, trace};
use serialport::SerialPort;
use std::io::{Error, ErrorKind, Read};

impl<T> Transceiver<T>
where
    T: SerialPort,
{
    /// Read an ASH [`Packet`].
    ///
    /// # Arguments
    /// * `buffer` The buffer used for input buffering.
    ///
    /// # Errors
    /// Returns an [`Error`] if any I/O, protocol or parsing error occurs.
    pub(in crate::transceiver) fn read_packet(&mut self) -> std::io::Result<Packet> {
        self.buffer_frame()?;
        Packet::try_from(self.buffers.frame.as_slice())
    }

    /// Reads an ASH frame into a [`FrameBuffer`].
    ///
    /// # Arguments
    /// * `buffer` The buffer used for input buffering.
    ///
    /// # Errors
    /// Returns an [`Error`] if any I/O or protocol error occurs.
    fn buffer_frame(&mut self) -> std::io::Result<()> {
        self.buffers.frame.clear();
        let serial_port = &mut self.serial_port;
        let mut error = false;

        for byte in serial_port.bytes() {
            match byte? {
                CANCEL => {
                    debug!("Resetting buffer due to cancel byte.");
                    trace!("Error condition: {error}");
                    trace!("Buffer: {:#04X?}", self.buffers.frame);
                    self.buffers.frame.clear();
                    error = false;
                }
                FLAG => {
                    debug!("Received flag byte.");

                    if !error && !self.buffers.frame.is_empty() {
                        debug!("Frame complete.");
                        trace!("Buffer: {:#04X?}", self.buffers.frame);
                        self.buffers.frame.unstuff();
                        trace!("Unstuffed buffer: {:#04X?}", self.buffers.frame);
                        return Ok(());
                    }

                    debug!("Resetting buffer due to error or empty buffer.");
                    trace!("Error condition: {error}");
                    trace!("Buffer: {:#04X?}", self.buffers.frame);
                    self.buffers.frame.clear();
                    error = false;
                }
                SUBSTITUTE => {
                    debug!("Received SUBSTITUTE byte. Setting error condition.");
                    error = true;
                }
                X_ON => {
                    debug!("NCP requested to resume transmission.");
                }
                X_OFF => {
                    debug!("NCP requested to stop transmission.");
                }
                WAKE => {
                    debug!("NCP tried to wake us up.");
                }
                byte => {
                    if self.buffers.frame.push(byte).is_err() {
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
    /// Returns an [`Error`](Error) if any I/O error occurs.
    pub(in crate::transceiver) fn write_frame<F>(&mut self, frame: &F) -> std::io::Result<()>
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
