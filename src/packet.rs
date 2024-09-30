use std::fmt::{Debug, Display, Formatter};

pub use ack::Ack;
pub use data::Data;
pub use error::Error;
pub use nak::Nak;
pub use rst::Rst;
pub use rst_ack::RstAck;

use crate::error::frame;

mod ack;
mod data;
mod error;
mod headers;
mod nak;
mod rst;
mod rst_ack;

#[allow(variant_size_differences)]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Packet {
    /// ACK frame
    Ack(Ack),
    /// Data frame
    Data(Data),
    /// Error frame
    Error(Error),
    /// NAK frame
    Nak(Nak),
    /// RST frame
    Rst(Rst),
    /// RST ACK frame
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

    use super::{headers, Packet};
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
                headers::Data::from_bits_retain(DATA[0]),
                DATA[1..DATA.len() - 3].iter().copied().collect()
            ))
        );
    }

    #[test]
    fn test_ack_try_from_bytes_slice() {
        const ACKS: [[u8; 4]; 2] = [[0x81, 0x60, 0x59, 0x7E], [0x8E, 0x91, 0xB6, 0x7E]];

        for ack in ACKS {
            let packet = Packet::try_from(&ack[..ack.len() - 1]).unwrap();
            assert_eq!(
                packet,
                Packet::Ack(Ack::new(headers::Ack::from_bits_retain(ack[0])))
            );
        }
    }

    #[test]
    fn test_nak_try_from_bytes_slice() {
        const NAKS: [[u8; 4]; 2] = [[0xA6, 0x34, 0xDC, 0x7E], [0xAD, 0x85, 0xB7, 0x7E]];

        for nak in NAKS {
            let packet = Packet::try_from(&nak[..nak.len() - 1]).unwrap();
            assert_eq!(
                packet,
                Packet::Nak(Nak::new(headers::Nak::from_bits_retain(nak[0])))
            );
        }
    }
}
