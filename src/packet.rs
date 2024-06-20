use std::fmt::{Debug, Display, Formatter};

pub use ack::Ack;
pub use data::{Data, MAX_PAYLOAD_SIZE, METADATA_SIZE, MIN_PAYLOAD_SIZE};
pub use error::Error;
pub use nak::Nak;
pub use rst::Rst;
pub use rst_ack::RstAck;

use crate::error::frame;
use crate::frame::Frame;

mod ack;
mod data;
mod error;
mod nak;
mod rst;
mod rst_ack;

// In the wost-case, all frame bytes are stuffed (*2) and we append the FLAG byte (+1).
const MAX_FRAME_SIZE: usize = (METADATA_SIZE + MAX_PAYLOAD_SIZE) * 2 + 1;

/// A stack-allocated buffer that can hold an `ASHv2` frame up to its maximum size.
pub type FrameBuffer = heapless::Vec<u8, MAX_FRAME_SIZE>;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Packet {
    Ack(Ack),
    Data(Data),
    Error(Error),
    Nak(Nak),
    Rst(Rst),
    RstAck(RstAck),
}

impl Display for Packet {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Ack(ack) => Display::fmt(ack, f),
            Self::Data(data) => Display::fmt(data, f),
            Self::Error(error) => Display::fmt(error, f),
            Self::Nak(nak) => Display::fmt(nak, f),
            Self::Rst(rst) => Display::fmt(rst, f),
            Self::RstAck(rst_ack) => Display::fmt(rst_ack, f),
        }
    }
}

impl Frame for Packet {
    fn header(&self) -> u8 {
        match self {
            Self::Ack(ack) => ack.header(),
            Self::Data(data) => data.header(),
            Self::Error(error) => error.header(),
            Self::Nak(nak) => nak.header(),
            Self::Rst(rst) => rst.header(),
            Self::RstAck(rst_ack) => rst_ack.header(),
        }
    }

    fn crc(&self) -> u16 {
        match self {
            Self::Ack(ack) => ack.crc(),
            Self::Data(data) => data.crc(),
            Self::Error(error) => error.crc(),
            Self::Nak(nak) => nak.crc(),
            Self::Rst(rst) => rst.crc(),
            Self::RstAck(rst_ack) => rst_ack.crc(),
        }
    }

    fn is_header_valid(&self) -> bool {
        match self {
            Self::Ack(ack) => ack.is_header_valid(),
            Self::Data(data) => data.is_header_valid(),
            Self::Error(error) => error.is_header_valid(),
            Self::Nak(nak) => nak.is_header_valid(),
            Self::Rst(rst) => rst.is_header_valid(),
            Self::RstAck(rst_ack) => rst_ack.is_header_valid(),
        }
    }

    fn is_crc_valid(&self) -> bool {
        match self {
            Self::Ack(ack) => ack.is_crc_valid(),
            Self::Data(data) => data.is_crc_valid(),
            Self::Error(error) => error.is_crc_valid(),
            Self::Nak(nak) => nak.is_crc_valid(),
            Self::Rst(rst) => rst.is_crc_valid(),
            Self::RstAck(rst_ack) => rst_ack.is_crc_valid(),
        }
    }

    fn is_valid(&self) -> bool {
        match self {
            Self::Ack(ack) => ack.is_valid(),
            Self::Data(data) => data.is_valid(),
            Self::Error(error) => error.is_valid(),
            Self::Nak(nak) => nak.is_valid(),
            Self::Rst(rst) => rst.is_valid(),
            Self::RstAck(rst_ack) => rst_ack.is_valid(),
        }
    }
}

impl TryFrom<&[u8]> for Packet {
    type Error = frame::Error;

    fn try_from(buffer: &[u8]) -> Result<Self, <Self as TryFrom<&[u8]>>::Error> {
        match *buffer
            .first()
            .ok_or(<Self as TryFrom<&[u8]>>::Error::InvalidHeader(None))?
        {
            rst::HEADER => Rst::try_from(buffer).map(Self::Rst),
            rst_ack::HEADER => RstAck::try_from(buffer).map(Self::RstAck),
            error::HEADER => Error::try_from(buffer).map(Self::Error),
            header => {
                if header & 0x80 == 0x00 {
                    Data::try_from(buffer).map(Self::Data)
                } else if header & 0x60 == 0x00 {
                    Ack::try_from(buffer).map(Self::Ack)
                } else if header & 0x60 == 0x20 {
                    Nak::try_from(buffer).map(Self::Nak)
                } else {
                    Err(<Self as TryFrom<&[u8]>>::Error::InvalidHeader(Some(header)))
                }
            }
        }
    }
}
