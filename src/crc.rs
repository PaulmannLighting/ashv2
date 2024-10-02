use crc::{Crc, CRC_16_IBM_3740};

/// CRC-16-IBM-3740 checksum function.
pub const CRC: Crc<u16> = Crc::<u16>::new(&CRC_16_IBM_3740);
