use crate::ash_write::AshWrite;
use crate::packet::{Ack, Data, Nak};
use crate::transceiver::Transceiver;
use crate::wrapping_u3::WrappingU3;
use std::io::{Error, ErrorKind};
use std::time::SystemTime;

impl Transceiver {
    pub(in crate::transceiver) fn ack(&mut self, ack_number: WrappingU3) -> std::io::Result<()> {
        self.send_ack(&Ack::create(ack_number, self.n_rdy()))
    }

    pub(in crate::transceiver) fn nak(&mut self) -> std::io::Result<()> {
        self.send_nak(&Nak::create(self.ack_number(), self.n_rdy()))
    }

    pub(in crate::transceiver) fn send_data(&mut self, data: Data) -> std::io::Result<()> {
        self.serial_port
            .write_frame_buffered(&data, &mut self.buffers.frame)?;
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
