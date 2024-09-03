use std::fmt::{Debug, Display};

use crate::protocol::Stuff;
use crate::{FrameBuffer, CRC};

pub trait Frame: Debug + Display {
    /// Returns the frame's header.
    fn header(&self) -> u8;

    /// Returns the CRC checksum.
    fn crc(&self) -> u16;

    /// Determines whether the CRC checksum is valid.
    fn is_crc_valid(&self) -> bool {
        self.crc() == self.calculate_crc()
    }

    /// Calculates the CRC checksum of the frame data.
    fn calculate_crc(&self) -> u16 {
        CRC.checksum(&[self.header()])
    }

    /// Returns the frame as a byte slice.
    fn bytes(&self) -> impl AsRef<[u8]>;

    /// Returns the frame's stuffed bytes in a [`FrameBuffer`].
    fn stuffed(&self) -> FrameBuffer {
        self.bytes().as_ref().iter().copied().stuff().collect()
    }
}
