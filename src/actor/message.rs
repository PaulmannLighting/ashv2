use crate::frame::{Error, Rst, RstAck};
use crate::utils::WrappingU3;

/// Messages sent to the `ASHv2` transmitter.
#[expect(variant_size_differences)]
#[derive(Debug)]
pub enum Message {
    /// Payload received from the network.
    Payload(Box<[u8]>),
    /// Send an ACK frame with the given frame number.
    Ack(WrappingU3),
    /// Send a NAK frame with the given expected frame number or the current frame counter.
    Nak(Option<WrappingU3>),
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
