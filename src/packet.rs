use crate::FrameError;
use std::fmt::{Debug, Display, Formatter};

pub mod ack;
pub mod data;
pub mod error;
pub mod nak;
pub mod rst;
pub mod rst_ack;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Packet {
    Ack(ack::Ack),
    Data(data::Data),
    Error(error::Error),
    Nak(nak::Nak),
    Rst(rst::Rst),
    RstAck(rst_ack::RstAck),
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

impl TryFrom<&[u8]> for Packet {
    type Error = FrameError;

    fn try_from(buffer: &[u8]) -> Result<Self, <Self as TryFrom<&[u8]>>::Error> {
        match *buffer
            .first()
            .ok_or(<Self as TryFrom<&[u8]>>::Error::InvalidHeader(None))?
        {
            rst::HEADER => Ok(Self::Rst(rst::Rst::try_from(buffer)?)),
            rst_ack::HEADER => Ok(Self::RstAck(rst_ack::RstAck::try_from(buffer)?)),
            error::HEADER => Ok(Self::Error(error::Error::try_from(buffer)?)),
            header => {
                if header & 0x80 == 0x00 {
                    Ok(Self::Data(data::Data::try_from(buffer)?))
                } else if header & 0x60 == 0x00 {
                    Ok(Self::Ack(ack::Ack::try_from(buffer)?))
                } else if header & 0x60 == 0x20 {
                    Ok(Self::Nak(nak::Nak::try_from(buffer)?))
                } else {
                    Err(<Self as TryFrom<&[u8]>>::Error::InvalidHeader(Some(header)))
                }
            }
        }
    }
}
