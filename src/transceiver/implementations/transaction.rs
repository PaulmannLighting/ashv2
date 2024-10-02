use crate::packet::Data;
use crate::transceiver::Transceiver;
use std::io::{Error, ErrorKind};
use std::slice::Chunks;

impl Transceiver {
    pub(in crate::transceiver) fn transaction(
        &mut self,
        mut chunks: Chunks<'_, u8>,
    ) -> std::io::Result<()> {
        self.state.within_transaction = true;

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
        while !self.buffers.retransmits.is_full() {
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
        self.buffers.payload.clear();
        self.buffers
            .payload
            .extend_from_slice(chunk)
            .map_err(|()| {
                Error::new(
                    ErrorKind::OutOfMemory,
                    "ASHv2: could not append chunk to frame buffer",
                )
            })?;
        let data = Data::create(self.state.next_frame_number(), self.buffers.payload.clone());
        self.write_frame(&data)
    }

    fn disable_callbacks(&mut self) -> std::io::Result<()> {
        self.ack(self.state.ack_number())
    }
}
