pub mod ack;
pub mod nak;
pub mod rst;
pub mod rst_ack;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Packet {
    Ack(ack::Ack),
    Nak(nak::Nak),
    Rst(rst::Rst),
    RstAck(rst_ack::RstAck),
    /*
    Data(data::Data),
    Error(error::Error),
    */
}
