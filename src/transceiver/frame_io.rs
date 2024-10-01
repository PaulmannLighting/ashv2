use crate::frame::Frame;
use crate::packet::Packet;
use crate::protocol::{Stuff, Unstuff, CANCEL, FLAG, SUBSTITUTE, WAKE, X_OFF, X_ON};
use crate::transceiver::Transceiver;
use log::{debug, trace};
use std::io::{Error, ErrorKind, Read, Write};

impl Transceiver {
    /// Read an ASH [`Packet`].
    ///
    /// # Arguments
    /// * `buffer` The buffer used for input buffering.
    ///
    /// # Errors
    /// Returns an [`Error`] if any I/O, protocol or parsing error occurs.
    pub(super) fn read_packet(&mut self) -> std::io::Result<Packet> {
        self.buffer_frame()?;
        Packet::try_from(self.frame_buffer.as_slice())
    }

    /// Reads an ASH frame into a [`FrameBuffer`].
    ///
    /// # Arguments
    /// * `buffer` The buffer used for input buffering.
    ///
    /// # Errors
    /// Returns an [`Error`] if any I/O or protocol error occurs.
    fn buffer_frame(&mut self) -> std::io::Result<()> {
        self.frame_buffer.clear();
        let serial_port = &mut self.serial_port;
        let mut error = false;

        for byte in serial_port.bytes() {
            match byte? {
                CANCEL => {
                    debug!("Resetting buffer due to cancel byte.");
                    trace!("Error condition: {error}");
                    trace!("Buffer: {:#04X?}", self.frame_buffer);
                    self.frame_buffer.clear();
                    error = false;
                }
                FLAG => {
                    debug!("Received flag byte.");

                    if !error && !self.frame_buffer.is_empty() {
                        debug!("Frame complete.");
                        trace!("Buffer: {:#04X?}", self.frame_buffer);
                        self.frame_buffer.unstuff();
                        trace!("Unstuffed buffer: {:#04X?}", self.frame_buffer);
                        return Ok(());
                    }

                    debug!("Resetting buffer due to error or empty buffer.");
                    trace!("Error condition: {error}");
                    trace!("Buffer: {:#04X?}", self.frame_buffer);
                    self.frame_buffer.clear();
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
                    if self.frame_buffer.push(byte).is_err() {
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
    pub(super) fn write_frame<T>(&mut self, frame: &T) -> std::io::Result<()>
    where
        T: Frame,
    {
        debug!("Writing frame: {frame}");
        trace!("{frame:#04X?}");
        self.frame_buffer.clear();
        frame.buffer(&mut self.frame_buffer).map_err(|()| {
            Error::new(
                ErrorKind::OutOfMemory,
                "could not append frame bytes to buffer",
            )
        })?;
        self.frame_buffer.stuff()?;
        self.frame_buffer
            .push(FLAG)
            .map_err(|_| Error::new(ErrorKind::OutOfMemory, "could not append flag byte"))?;
        trace!("Writing bytes: {:#04X?}", self.frame_buffer);
        self.serial_port.write_all(&self.frame_buffer)?;
        self.serial_port.flush()
    }
}
