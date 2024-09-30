mod buffered;
mod rw_frame;

use crate::FrameBuffer;
use serialport::TTYPort;

#[derive(Debug)]
pub struct Transceiver {
    serial_port: TTYPort,
    frame_buffer: FrameBuffer,
}
