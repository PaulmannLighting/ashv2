use crate::utils::WrappingU3;
use crate::Payload;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Response {
    Data(Payload),
    Ack(WrappingU3),
    Nak(WrappingU3),
}
