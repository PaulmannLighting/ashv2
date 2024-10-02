use crate::packet::Data;
use crate::Transceiver;
use log::{debug, warn};
use serialport::SerialPort;
use std::io::{Error, ErrorKind};
use std::slice::Chunks;

impl<T> Transceiver<T>
where
    T: SerialPort,
{
    pub(in crate::transceiver) fn transaction(
        &mut self,
        mut chunks: Chunks<'_, u8>,
    ) -> std::io::Result<()> {
        debug!("Starting transaction.");
        self.state.within_transaction = true;

        // Make sure that we do not receive any callbacks during the transaction.
        self.clear_callbacks()?;

        while self.send_chunks(&mut chunks)? {
            // Handle responses to sent chunks.
            while let Some(packet) = self.receive()? {
                self.handle_packet(packet)?;
            }
        }

        // Handle any remaining responses.
        while let Some(packet) = self.receive()? {
            self.handle_packet(packet)?;
        }

        // Wait for retransmits to finish.
        while !self.buffers.retransmits.is_empty() {
            self.retransmit_timed_out_data()?;

            while let Some(packet) = self.receive()? {
                self.handle_packet(packet)?;
            }
        }

        self.send_response()?;
        debug!("Transaction completed.");
        self.state.within_transaction = false;
        // Send ACK without `nRDY` set, to re-enable callbacks.
        self.ack()?;
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
        let payload: heapless::Vec<u8, { Data::MAX_PAYLOAD_SIZE }> =
            chunk.try_into().map_err(|()| {
                Error::new(
                    ErrorKind::OutOfMemory,
                    "ASHv2: could not append chunk to frame buffer",
                )
            })?;
        let data = Data::new(
            self.state.next_frame_number(),
            payload,
            self.state.ack_number(),
        );
        self.send_data(data)
    }

    fn clear_callbacks(&mut self) -> std::io::Result<()> {
        // Disable callbacks by sending an ACK with `nRDY` set.
        self.ack()?;

        while let Some(packet) = self.receive()? {
            self.handle_packet(packet)?;
        }

        // Any data we received can not be a response to our transaction.
        if self.buffers.response.is_empty() {
            return Ok(());
        }

        warn!("Received data before beginning transaction. Forwarding to callback channel.");
        self.channels
            .callback(self.buffers.response.clone().into())?;
        self.buffers.response.clear();
        Ok(())
    }

    fn send_response(&mut self) -> std::io::Result<()> {
        self.channels
            .respond(Ok(self.buffers.response.clone().into()))?;
        self.buffers.clear();
        Ok(())
    }
}
