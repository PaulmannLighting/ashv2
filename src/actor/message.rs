use std::io;

use tokio::sync::oneshot::Sender;

use crate::Payload;
use crate::frame::{Error, Rst, RstAck};
use crate::utils::WrappingU3;

/// Messages sent to the `ASHv2` transmitter.
#[expect(variant_size_differences)]
#[derive(Debug)]
pub enum Message {
    /// Payload received from the network.
    Payload {
        /// Data payload to send.
        payload: Box<Payload>,
        /// Response channel to notify when the payload has been sent.
        response: Sender<io::Result<()>>,
    },
    /// Send an ACK frame with the given ack number.
    Ack(WrappingU3),
    /// Send a NAK frame with the given ack number.
    Nak(WrappingU3),
    /// Received RST frame.
    Rst(Rst),
    /// Received RST-ACK frame.
    RstAck(RstAck),
    /// Received ERROR frame.
    Error(Error),
    /// Acknowledgement sent frames up to the given frame number.
    AckSentFrame(WrappingU3),
    /// Negative Acknowledgement sent frames up to the given frame number.
    NakSentFrame(WrappingU3),
}
