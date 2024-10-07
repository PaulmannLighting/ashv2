use crate::Request;
use std::io::ErrorKind;
use std::sync::mpsc::SyncSender;
use tokio_util::bytes::BytesMut;
use tokio_util::codec::Decoder;

/// A host that communicates with the NCP.
#[derive(Debug)]
pub struct Host {
    sender: SyncSender<Request>,
    buffer: BytesMut,
}

impl Host {
    /// Create a new host.
    #[must_use]
    pub fn new(sender: SyncSender<Request>) -> Self {
        Self {
            sender,
            buffer: BytesMut::new(),
        }
    }

    /// Communicate with the NCP.
    ///
    /// Sends a payload request to the NCP and returns the decoded response.
    ///
    /// # Errors
    ///
    /// Returns a [`Decoder::Error`] if the request could not be sent
    /// or the response could not be received or decoded.
    pub fn communicate<T>(&mut self, payload: &[u8]) -> Result<T::Item, T::Error>
    where
        T: Decoder + Default,
    {
        self.buffer.clear();

        let (request, response) = Request::new(payload.into());
        self.sender.send(request).map_err(|_| {
            std::io::Error::new(ErrorKind::BrokenPipe, "ASHv2: Failed to send request.")
        })?;

        let mut decoder = T::default();

        loop {
            if self.buffer.is_empty() {
                self.buffer.extend_from_slice(&response.recv().map_err(|_| {
                    std::io::Error::new(
                        ErrorKind::BrokenPipe,
                        "ASHv2: Response channel disconnected.",
                    )
                })?);
            }

            if let Some(item) = decoder.decode(&mut self.buffer)? {
                self.buffer.clear();
                return Ok(item);
            }
        }
    }
}
