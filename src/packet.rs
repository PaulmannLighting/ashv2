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
    InvalidHeader,
    BufferTooSmall,
}

impl TryFrom<&[u8]> for Packet {
    type Error = Error;

    fn try_from(buffer: &[u8]) -> Result<Self, Error> {
        if buffer.len() < 4 {
            return Err(Error::BufferTooSmall);
        }

        let header = *buffer.get(0).ok_or(Error::MissingHeader)?;

        if header & 0x80 == 0 {
            return Ok(Self::Data(data::Data::new(
                header,
                buffer[1..(buffer.len() - 3)].into(),
                u16::from_be_bytes([buffer[buffer.len() - 3], buffer[buffer.len() - 2]]),
                buffer[buffer.len() - 1],
            )));
        }

        return Err(Error::InvalidHeader);
    }
}
