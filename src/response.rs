use crate::packet::{Ack, Data, Nak};

/// A response from the receiver.
#[derive(Debug)]
pub enum Response {
    Ack(Ack),
    Nak(Nak),
    Data(Data),
}
