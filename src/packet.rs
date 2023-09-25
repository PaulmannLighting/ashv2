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

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Error {
    MissingHeader,
    InvalidHeader(u8),
    BufferTooSmall(usize),
    InvalidBufferSize { expected: usize, found: usize },
}

impl TryFrom<&[u8]> for Packet {
    type Error = Error;

    fn try_from(buffer: &[u8]) -> Result<Self, Error> {
        match *buffer.first().ok_or(Error::MissingHeader)? {
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
                    Err(Error::InvalidHeader(header))
                }
            }
        }
    }
}
