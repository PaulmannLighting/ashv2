use crate::ash_read::AshRead;
use crate::ash_write::AshWrite;
use crate::frame::Frame;
use crate::packet::Packet;
use crate::transceiver::Transceiver;
use std::io::ErrorKind;

pub trait RwFrame {
    /// Reads a frame from the serial port.
    fn read_frame(&mut self) -> std::io::Result<Option<Packet>>;

    /// Reads a frame from the serial port.
    fn write_frame<T>(&mut self, frame: T) -> std::io::Result<()>
    where
        T: Frame;
}

impl RwFrame for Transceiver {
    fn read_frame(&mut self) -> std::io::Result<Option<Packet>> {
        self.serial_port
            .read_packet_buffered(&mut self.frame_buffer)
            .map(Some)
            .or_else(|error| {
                if error.kind() == ErrorKind::TimedOut {
                    Ok(None)
                } else {
                    Err(error)
                }
            })
    }

    fn write_frame<U>(&mut self, frame: U) -> std::io::Result<()>
    where
        U: Frame,
    {
        self.serial_port
            .write_frame_buffered(&frame, &mut self.frame_buffer)
    }
}
