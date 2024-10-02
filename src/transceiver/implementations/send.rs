use crate::packet::{Ack, Data, Nak, RST};
use crate::transceiver::Transceiver;
use crate::wrapping_u3::WrappingU3;
use std::io::{Error, ErrorKind};
use std::time::SystemTime;

impl Transceiver {
    /// Send an ACK frame with the given ACK number.
    pub(in crate::transceiver) fn ack(&mut self, ack_number: WrappingU3) -> std::io::Result<()> {
        self.send_ack(&Ack::create(ack_number, self.state.n_rdy()))
    }

    /// Send a NAK frame with the current ACK number.
    pub(in crate::transceiver) fn nak(&mut self) -> std::io::Result<()> {
        self.send_nak(&Nak::create(self.state.ack_number(), self.state.n_rdy()))
    }

    /// Send a RST frame.
    pub(in crate::transceiver) fn rst(&mut self) -> std::io::Result<()> {
        self.write_frame(&RST)
    }

    /// Send a data frame.
    pub(in crate::transceiver) fn send_data(&mut self, data: Data) -> std::io::Result<()> {
        self.write_frame(&data)?;
        self.enqueue_retransmit(data)
    }

    fn send_ack(&mut self, ack: &Ack) -> std::io::Result<()> {
        if ack.not_ready() {
            self.state
                .last_n_rdy_transmission
                .replace(SystemTime::now());
        }

        self.write_frame(ack)
    }

    fn send_nak(&mut self, nak: &Nak) -> std::io::Result<()> {
        if nak.not_ready() {
            self.state
                .last_n_rdy_transmission
                .replace(SystemTime::now());
        }

        self.write_frame(nak)
    }

    fn enqueue_retransmit(&mut self, data: Data) -> std::io::Result<()> {
        self.buffers
            .retransmits
            .insert(0, data.into())
            .map_err(|_| {
                Error::new(
                    ErrorKind::OutOfMemory,
                    "ASHv2: failed to enqueue retransmit",
                )
            })
    }
}
