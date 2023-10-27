mod ack;
mod data;
mod error;
mod nak;
mod rst;
mod rst_ack;

use crate::frame::Frame;
use crate::FrameError;
pub use ack::Ack;
pub use data::{Data, MAX_PAYLOAD_SIZE, MIN_PAYLOAD_SIZE};
pub use error::Error;
pub use nak::Nak;
pub use rst::Rst;
pub use rst_ack::RstAck;
use std::fmt::{Debug, Display, Formatter};

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

impl<'a> IntoIterator for &'a Packet {
    type Item = u8;
    type IntoIter = Box<dyn Iterator<Item = u8> + 'a>;

    fn into_iter(self) -> Self::IntoIter {
        match self {
            Packet::Ack(ack) => Box::new(ack.into_iter()),
            Packet::Data(data) => Box::new(data.into_iter()),
            Packet::Error(error) => Box::new(error.into_iter()),
            Packet::Nak(nak) => Box::new(nak.into_iter()),
            Packet::Rst(rst) => Box::new(rst.into_iter()),
            Packet::RstAck(rst_ack) => Box::new(rst_ack.into_iter()),
        }
    }
}

impl TryFrom<&[u8]> for Packet {
    type Error = FrameError;

    fn try_from(buffer: &[u8]) -> Result<Self, <Self as TryFrom<&[u8]>>::Error> {
        match *buffer
            .first()
            .ok_or(<Self as TryFrom<&[u8]>>::Error::InvalidHeader(None))?
        {
            rst::HEADER => Ok(Self::Rst(Rst::try_from(buffer)?)),
            rst_ack::HEADER => Ok(Self::RstAck(RstAck::try_from(buffer)?)),
            error::HEADER => Ok(Self::Error(Error::try_from(buffer)?)),
            header => {
                if header & 0x80 == 0x00 {
                    Ok(Self::Data(Data::try_from(buffer)?))
                } else if header & 0x60 == 0x00 {
                    Ok(Self::Ack(Ack::try_from(buffer)?))
                } else if header & 0x60 == 0x20 {
                    Ok(Self::Nak(Nak::try_from(buffer)?))
                } else {
                    Err(<Self as TryFrom<&[u8]>>::Error::InvalidHeader(Some(header)))
                }
            }
        }
    }
}
