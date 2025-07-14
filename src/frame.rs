//! Frame types and their respective headers for the `ASHv2` protocol.

use std::fmt::{Debug, Display, Formatter, LowerHex, UpperHex};
use std::io::ErrorKind;

use crate::validate::Validate;

pub use ack::Ack;
pub use data::Data;
pub use error::Error;
pub use nak::Nak;
pub use rst::{RST, Rst};
pub use rst_ack::RstAck;

mod ack;
mod data;
mod error;
pub mod headers;
mod nak;
mod rst;
mod rst_ack;

/// Available frame types.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Frame {
    /// `ACK` frame
    Ack(Ack),
    /// `DATA` frame
    Data(Box<Data>),
    /// `ERROR` frame
    Error(Error),
    /// `NAK` frame
    Nak(Nak),
    /// `RST` frame
    Rst(Rst),
    /// `RST_ACK` frame
    RstAck(RstAck),
}

impl Display for Frame {
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

impl TryFrom<&[u8]> for Frame {
    type Error = std::io::Error;

    fn try_from(buffer: &[u8]) -> std::io::Result<Self> {
        match *buffer
            .first()
            .ok_or_else(|| std::io::Error::new(ErrorKind::UnexpectedEof, "Missing frame header."))?
        {
            Rst::HEADER => Rst::try_from(buffer).map(Self::Rst),
            RstAck::HEADER => RstAck::try_from(buffer).map(Self::RstAck),
            Error::HEADER => Error::try_from(buffer).map(Self::Error),
            header if header & 0x80 == 0x00 => {
                Data::try_from(buffer).map(|data| Self::Data(data.into()))
            }
            header if header & 0x60 == 0x00 => Ack::try_from(buffer).map(Self::Ack),
            header if header & 0x60 == 0x20 => Nak::try_from(buffer).map(Self::Nak),
            header => Err(std::io::Error::new(
                ErrorKind::InvalidData,
                format!("Unknown frame header: {header:#04X}"),
            )),
        }
    }
}

impl LowerHex for Frame {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Ack(ack) => LowerHex::fmt(ack, f),
            Self::Data(data) => LowerHex::fmt(data.as_ref(), f),
            Self::Error(error) => LowerHex::fmt(error, f),
            Self::Nak(nak) => LowerHex::fmt(nak, f),
            Self::Rst(rst) => LowerHex::fmt(rst, f),
            Self::RstAck(rst_ack) => LowerHex::fmt(rst_ack, f),
        }
    }
}

impl UpperHex for Frame {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Ack(ack) => UpperHex::fmt(ack, f),
            Self::Data(data) => UpperHex::fmt(data.as_ref(), f),
            Self::Error(error) => UpperHex::fmt(error, f),
            Self::Nak(nak) => UpperHex::fmt(nak, f),
            Self::Rst(rst) => UpperHex::fmt(rst, f),
            Self::RstAck(rst_ack) => UpperHex::fmt(rst_ack, f),
        }
    }
}

impl Validate for Frame {
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
}

#[allow(clippy::unwrap_used)]
#[cfg(test)]
mod tests {
    use super::{Frame, Rst};
    use crate::code::Code;
    use crate::validate::Validate;

    #[test]
    fn test_rst_try_from_bytes_slice() {
        const RST: [u8; 4] = [0xC0, 0x38, 0xBC, 0x7E];
        let packet = Frame::try_from(&RST[..RST.len() - 1]).unwrap();
        assert_eq!(packet, Frame::Rst(Rst::new()));
    }

    #[test]
    fn test_rstack_try_from_bytes_slice() {
        const RST_ACK: [u8; 6] = [0xC1, 0x02, 0x02, 0x9B, 0x7B, 0x7E];

        match Frame::try_from(&RST_ACK[..RST_ACK.len() - 1]).unwrap() {
            Frame::RstAck(rst_ack) => {
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

        match Frame::try_from(&ERROR[..ERROR.len() - 1]).unwrap() {
            Frame::Error(error) => {
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

        match Frame::try_from(&DATA[..DATA.len() - 1]).unwrap() {
            Frame::Data(data) => {
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

        match Frame::try_from(&ack[..ack.len() - 1]).unwrap() {
            Frame::Ack(ack) => {
                assert!(!ack.not_ready());
                assert_eq!(ack.ack_num(), 1);
                assert_eq!(ack.crc(), 0x6059);
                assert!(ack.is_crc_valid());
            }
            packet => panic!("Expected Ack, got {packet:?}"),
        }

        let ack = [0x8E, 0x91, 0xB6, 0x7E];

        match Frame::try_from(&ack[..ack.len() - 1]).unwrap() {
            Frame::Ack(ack) => {
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

        match Frame::try_from(&nak[..nak.len() - 1]).unwrap() {
            Frame::Nak(nak) => {
                assert!(!nak.not_ready());
                assert_eq!(nak.ack_num(), 0x06);
                assert_eq!(nak.crc(), 0x34DC);
                assert!(nak.is_crc_valid());
            }
            packet => panic!("Expected Nak, got {packet:?}"),
        }

        let nak = [0xAD, 0x85, 0xB7, 0x7E];

        match Frame::try_from(&nak[..nak.len() - 1]).unwrap() {
            Frame::Nak(nak) => {
                assert!(nak.not_ready());
                assert_eq!(nak.ack_num(), 0x05);
                assert_eq!(nak.crc(), 0x85B7);
                assert!(nak.is_crc_valid());
            }
            packet => panic!("Expected Nak, got {packet:?}"),
        }
    }
}
