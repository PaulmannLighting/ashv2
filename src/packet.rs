pub mod ack;
pub mod nak;
pub mod rst;

pub enum Packet {
    Ack(ack::Ack),
    Nak(nak::Nak),
    Rst(rst::Rst),
    /*
    Data(data::Data),
    Error(error::Error),
    RstAck(rst_ack::RstAck),
    */
}
