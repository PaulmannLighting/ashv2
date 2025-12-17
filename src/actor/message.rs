use crate::Payload;

pub enum Message {
    Payload(Payload),
    Nak(u8),
    Ack(u8),
    AckSentFrame(u8),
    NakSentFrame(u8),
    Rst,
}
