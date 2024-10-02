use crate::ash_write::AshWrite;
use crate::packet::Data;
use crate::transceiver::Transceiver;
use log::warn;
use std::io::{Error, ErrorKind};
use std::slice::Chunks;

impl Transceiver {
    pub(in crate::transceiver) fn transaction(
        &mut self,
        mut chunks: Chunks<'_, u8>,
    ) -> std::io::Result<()> {
        self.within_transaction = true;

        // Make sure that we do not receive any callbacks during the transaction.
        self.disable_callbacks()?;

        loop {
            if !self.send_chunks(&mut chunks)? {
                break;
            }

            self.receive()?;
        }

        Ok(())
    }

    /// Sends chunks as long as the retransmit queue is not full.
    fn send_chunks(&mut self, chunks: &mut Chunks<'_, u8>) -> std::io::Result<bool> {
        while !self.retransmits.is_full() {
            if let Some(chunk) = chunks.next() {
                self.send_chunk(chunk)?;
            } else {
                return Ok(false);
            }
        }

        Ok(true)
    }

    /// Sends a chunk of data.
    fn send_chunk(&mut self, chunk: &[u8]) -> std::io::Result<()> {
        self.payload_buffer.clear();
        self.payload_buffer.extend_from_slice(chunk).map_err(|()| {
            Error::new(
                ErrorKind::OutOfMemory,
                "ASHv2: could not append chunk to frame buffer",
            )
        })?;
        let data = Data::create(self.next_frame_number(), self.payload_buffer.clone());
        self.serial_port
            .write_frame_buffered(&data, &mut self.frame_buffer)
    }

    fn disable_callbacks(&mut self) -> std::io::Result<()> {
        self.ack(self.ack_number())
    }
}
