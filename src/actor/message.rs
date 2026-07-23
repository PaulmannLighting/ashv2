use std::fmt::Display;
use std::io;

use tokio::sync::oneshot::Sender;

use crate::Payload;
use crate::frame::{Error, Rst, RstAck};
use crate::hex_slice::HexSlice;

/// Messages sent to the `ASHv2` transmitter.
#[derive(Debug)]
#[cfg_attr(target_pointer_width = "64", expect(variant_size_differences))]
pub enum Message {
    /// Payload received from the network.
    Payload {
        /// Data payload to send.
        payload: Box<Payload>,
        /// Response channel to notify when the payload has been sent.
        response_tx: Sender<io::Result<()>>,
    },

    /// Send an ACK frame with the given ack number.
    Ack(u8),

    /// Send a NAK frame with the given ack number.
    Nak(u8),

    /// Received RST frame.
    Rst(Rst),

    /// Received RST-ACK frame.
    RstAck(RstAck),

    /// Received ERROR frame.
    Error(Error),

    /// Acknowledgement sent frames up to the given frame number.
    AckSentFrame(u8),

    /// Negative Acknowledgement sent frames up to the given frame number.
    NakSentFrame(u8),

    /// Terminate the receiver.
    Terminate,
}

impl Display for Message {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Payload { payload, .. } => write!(f, "Payload({:#04X})", HexSlice::new(payload)),
            Self::Ack(ack_num) => write!(f, "Ack({ack_num})"),
            Self::Nak(ack_num) => write!(f, "Nak({ack_num})"),
            Self::Rst(rst) => write!(f, "Rst({rst})"),
            Self::RstAck(rst_ack) => write!(f, "RstAck({rst_ack})"),
            Self::Error(error) => write!(f, "Error({error})"),
            Self::AckSentFrame(ack_num) => write!(f, "AckSentFrame({ack_num})"),
            Self::NakSentFrame(ack_num) => write!(f, "NakSentFrame({ack_num})"),
            Self::Terminate => write!(f, "Terminate"),
        }
    }
}
