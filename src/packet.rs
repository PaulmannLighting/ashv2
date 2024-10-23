use std::fmt::{Debug, Display, Formatter, LowerHex, UpperHex};
use std::io::ErrorKind;

pub use ack::Ack;
pub use data::Data;
pub use error::Error;
pub use nak::Nak;
pub use rst::{Rst, RST};
pub use rst_ack::RstAck;

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
    type Error = std::io::Error;

    fn try_from(buffer: &[u8]) -> std::io::Result<Self> {
        match *buffer.first().ok_or_else(|| {
            std::io::Error::new(ErrorKind::UnexpectedEof, "Missing packet header.")
        })? {
            Rst::HEADER => Rst::try_from(buffer).map(Self::Rst),
            RstAck::HEADER => RstAck::try_from(buffer).map(Self::RstAck),
            Error::HEADER => Error::try_from(buffer).map(Self::Error),
            header if header & 0x80 == 0x00 => Data::try_from(buffer).map(Self::Data),
            header if header & 0x60 == 0x00 => Ack::try_from(buffer).map(Self::Ack),
            header if header & 0x60 == 0x20 => Nak::try_from(buffer).map(Self::Nak),
            header => Err(std::io::Error::new(
                ErrorKind::InvalidData,
                format!("Unknown packet header: {header:#04X}"),
            )),
        }
    }
}

impl LowerHex for Packet {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Ack(ack) => LowerHex::fmt(ack, f),
            Self::Data(data) => LowerHex::fmt(data, f),
            Self::Error(error) => LowerHex::fmt(error, f),
            Self::Nak(nak) => LowerHex::fmt(nak, f),
            Self::Rst(rst) => LowerHex::fmt(rst, f),
            Self::RstAck(rst_ack) => LowerHex::fmt(rst_ack, f),
        }
    }
}

impl UpperHex for Packet {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Ack(ack) => UpperHex::fmt(ack, f),
            Self::Data(data) => UpperHex::fmt(data, f),
            Self::Error(error) => UpperHex::fmt(error, f),
            Self::Nak(nak) => UpperHex::fmt(nak, f),
            Self::Rst(rst) => UpperHex::fmt(rst, f),
            Self::RstAck(rst_ack) => UpperHex::fmt(rst_ack, f),
        }
    }
}

#[allow(clippy::unwrap_used)]
#[cfg(test)]
mod tests {
    use super::{Packet, Rst};
    use crate::code::Code;
    use crate::frame::Frame;

    #[test]
    fn test_rst_try_from_bytes_slice() {
        const RST: [u8; 4] = [0xC0, 0x38, 0xBC, 0x7E];
        let packet = Packet::try_from(&RST[..RST.len() - 1]).unwrap();
        assert_eq!(packet, Packet::Rst(Rst::new()));
    }

    #[test]
    fn test_rstack_try_from_bytes_slice() {
        const RST_ACK: [u8; 6] = [0xC1, 0x02, 0x02, 0x9B, 0x7B, 0x7E];

        match Packet::try_from(&RST_ACK[..RST_ACK.len() - 1]).unwrap() {
            Packet::RstAck(rst_ack) => {
                assert!(rst_ack.is_ash_v2());
                assert_eq!(rst_ack.version(), 2);
                assert_eq!(rst_ack.code(), Ok(Code::PowerOn));
                assert_eq!(rst_ack.crc(), 0x9B7B);
            }
            packet => panic!("Expected RstAck, got {packet:?}"),
        }
    }

    #[test]
    fn test_error_try_from_bytes_slice() {
        const ERROR: [u8; 6] = [0xC2, 0x02, 0x52, 0x98, 0xDE, 0x7E];

        match Packet::try_from(&ERROR[..ERROR.len() - 1]).unwrap() {
            Packet::Error(error) => {
                assert_eq!(error.version(), 2);
                assert_eq!(error.code(), Err(0x52));
                assert_eq!(error.crc(), 0x98DE);
            }
            packet => panic!("Expected Error, got {packet:?}"),
        }
    }

    #[test]
    fn test_data_try_from_bytes_slice() {
        const DATA: [u8; 11] = [
            0x53, 0x00, 0x80, 0x00, 0x02, 0x02, 0x11, 0x30, 0x63, 0x16, 0x7E,
        ];

        match Packet::try_from(&DATA[..DATA.len() - 1]).unwrap() {
            Packet::Data(data) => {
                assert_eq!(data.crc(), 0x6316);
                assert!(data.is_crc_valid());
                assert_eq!(data.into_payload().as_slice(), &DATA[1..DATA.len() - 3]);
            }
            packet => panic!("Expected Data, got {packet:?}"),
        }
    }

    #[test]
    fn test_ack_try_from_bytes_slice() {
        let ack = [0x81, 0x60, 0x59, 0x7E];

        match Packet::try_from(&ack[..ack.len() - 1]).unwrap() {
            Packet::Ack(ack) => {
                assert!(!ack.not_ready());
                assert_eq!(ack.ack_num(), 1);
                assert_eq!(ack.crc(), 0x6059);
                assert!(ack.is_crc_valid());
            }
            packet => panic!("Expected Ack, got {packet:?}"),
        }

        let ack = [0x8E, 0x91, 0xB6, 0x7E];

        match Packet::try_from(&ack[..ack.len() - 1]).unwrap() {
            Packet::Ack(ack) => {
                assert!(ack.not_ready());
                assert_eq!(ack.ack_num(), 0x06);
                assert_eq!(ack.crc(), 0x91B6);
                assert!(ack.is_crc_valid());
            }
            packet => panic!("Expected Ack, got {packet:?}"),
        }
    }

    #[test]
    fn test_nak_try_from_bytes_slice() {
        let nak = [0xA6, 0x34, 0xDC, 0x7E];

        match Packet::try_from(&nak[..nak.len() - 1]).unwrap() {
            Packet::Nak(nak) => {
                assert!(!nak.not_ready());
                assert_eq!(nak.ack_num(), 0x06);
                assert_eq!(nak.crc(), 0x34DC);
                assert!(nak.is_crc_valid());
            }
            packet => panic!("Expected Nak, got {packet:?}"),
        }

        let nak = [0xAD, 0x85, 0xB7, 0x7E];

        match Packet::try_from(&nak[..nak.len() - 1]).unwrap() {
            Packet::Nak(nak) => {
                assert!(nak.not_ready());
                assert_eq!(nak.ack_num(), 0x05);
                assert_eq!(nak.crc(), 0x85B7);
                assert!(nak.is_crc_valid());
            }
            packet => panic!("Expected Nak, got {packet:?}"),
        }
    }
}
