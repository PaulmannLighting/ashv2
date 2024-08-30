use std::fmt::{Debug, Display, Formatter};

pub use ack::Ack;
pub use data::Data;
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
const MAX_FRAME_SIZE: usize = (Data::METADATA_SIZE + Data::MAX_PAYLOAD_SIZE) * 2 + 1;

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

    fn calculate_crc(&self) -> u16 {
        match self {
            Self::Ack(ack) => ack.calculate_crc(),
            Self::Data(data) => data.calculate_crc(),
            Self::Error(error) => error.calculate_crc(),
            Self::Nak(nak) => nak.calculate_crc(),
            Self::Rst(rst) => rst.calculate_crc(),
            Self::RstAck(rst_ack) => rst_ack.calculate_crc(),
        }
    }

    fn bytes(&self) -> impl AsRef<[u8]> {
        match self {
            Self::Ack(ack) => Box::<[u8]>::from(ack.bytes().as_ref()),
            Self::Data(data) => Box::<[u8]>::from(data.bytes().as_ref()),
            Self::Error(error) => Box::<[u8]>::from(error.bytes().as_ref()),
            Self::Nak(nak) => Box::<[u8]>::from(nak.bytes().as_ref()),
            Self::Rst(rst) => Box::<[u8]>::from(rst.bytes().as_ref()),
            Self::RstAck(rst_ack) => Box::<[u8]>::from(rst_ack.bytes().as_ref()),
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
            Rst::HEADER => Rst::try_from(buffer).map(Self::Rst),
            RstAck::HEADER => RstAck::try_from(buffer).map(Self::RstAck),
            Error::HEADER => Error::try_from(buffer).map(Self::Error),
            header if header & 0x80 == 0x00 => Data::try_from(buffer).map(Self::Data),
            header if header & 0x60 == 0x00 => Ack::try_from(buffer).map(Self::Ack),
            header if header & 0x60 == 0x20 => Nak::try_from(buffer).map(Self::Nak),
            header => Err(<Self as TryFrom<&[u8]>>::Error::InvalidHeader(Some(header))),
        }
    }
}

#[allow(clippy::unwrap_used)]
#[cfg(test)]
mod tests {
    use crate::code::Code;

    use super::Packet;
    use super::{Ack, Data, Error, Nak, Rst};

    #[test]
    fn test_rst_try_from_bytes_slice() {
        const RST: [u8; 4] = [0xC0, 0x38, 0xBC, 0x7E];
        let packet = Packet::try_from(&RST[..RST.len() - 1]).unwrap();
        assert_eq!(packet, Packet::Rst(Rst::default()));
    }

    #[test]
    fn test_rstack_try_from_bytes_slice() {
        const RST_ACK: [u8; 6] = [0xC1, 0x02, 0x02, 0x9B, 0x7B, 0x7E];
        let packet = Packet::try_from(&RST_ACK[..RST_ACK.len() - 1]).unwrap();
        assert_eq!(packet, Packet::RstAck(Code::PowerOn.into()));
    }

    #[test]
    fn test_error_try_from_bytes_slice() {
        const ERROR: [u8; 6] = [0xC2, 0x02, 0x52, 0x98, 0xDE, 0x7E];
        let packet = Packet::try_from(&ERROR[..ERROR.len() - 1]).unwrap();
        assert_eq!(packet, Packet::Error(Error::new(ERROR[2])));
    }

    #[test]
    fn test_data_try_from_bytes_slice() {
        const DATA: [u8; 11] = [
            0x53, 0x00, 0x80, 0x00, 0x02, 0x02, 0x11, 0x30, 0x63, 0x16, 0x7E,
        ];
        let packet = Packet::try_from(&DATA[..DATA.len() - 1]).unwrap();
        assert_eq!(
            packet,
            Packet::Data(Data::new(
                DATA[0],
                DATA[1..DATA.len() - 3].iter().copied().collect()
            ))
        );
    }

    #[test]
    fn test_ack_try_from_bytes_slice() {
        const ACKS: [[u8; 4]; 2] = [[0x81, 0x60, 0x59, 0x7E], [0x8E, 0x91, 0xB6, 0x7E]];

        for ack in ACKS {
            let packet = Packet::try_from(&ack[..ack.len() - 1]).unwrap();
            assert_eq!(packet, Packet::Ack(Ack::new(ack[0])));
        }
    }

    #[test]
    fn test_nak_try_from_bytes_slice() {
        const NAKS: [[u8; 4]; 2] = [[0xA6, 0x34, 0xDC, 0x7E], [0xAD, 0x85, 0xB7, 0x7E]];

        for nak in NAKS {
            let packet = Packet::try_from(&nak[..nak.len() - 1]).unwrap();
            assert_eq!(packet, Packet::Nak(Nak::new(nak[0])));
        }
    }
}
