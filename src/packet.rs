mod ack;
mod data;
mod error;
mod nak;
mod rst;
mod rst_ack;

use crate::FrameError;
pub use ack::Ack;
pub use data::Data;
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
