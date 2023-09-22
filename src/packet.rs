pub mod ack;
pub mod nak;

pub enum Packet {
    Ack(ack::Ack),
    Nak(nak::Nak),
    /*
    Data(data::Data),
    Error(error::Error),
    Rst(rst::Rst),
    RstAck(rst_ack::RstAck),
    */
}
