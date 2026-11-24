//! Transmit-side frame buffer for `ASHv2` serial output.

use core::fmt::Display;
use std::io::{self, Error};

use log::debug;
use tokio::io::{AsyncWrite, AsyncWriteExt};

use super::{T_RX_ACK_MAX, Transmission};
use crate::actor::timer::timeout;
use crate::frame::RST;
use crate::protocol::{ControlByte, Stuff};
use crate::types::RawFrame;

/// Transmit-side buffer that encodes `ASHv2` frames for serial writes.
#[derive(Debug)]
pub struct Buffer<T> {
    /// Async writer for the serial transmit path.
    inner: T,
    /// Reusable frame buffer used for stuffing and termination.
    frame: RawFrame,
}

impl<T> Buffer<T> {
    /// Create a new transmit buffer around an async writer.
    #[must_use]
    pub const fn new(inner: T) -> Self {
        Self {
            inner,
            frame: RawFrame::new(),
        }
    }
}

impl<T> Buffer<T>
where
    T: AsyncWrite + Unpin,
{
    /// Writes the reset frame after a Cancel byte to discard any partial frame at the NCP.
    pub async fn write_reset(&mut self) -> io::Result<()> {
        timeout(
            T_RX_ACK_MAX,
            self.inner.write_u8(ControlByte::Cancel.into()),
        )
        .await??;
        self.write_frame(RST).await
    }

    /// Writes a DATA frame and records when the transport accepted it.
    pub async fn write_data(
        &mut self,
        transmission: &mut Transmission,
        ack_num: u8,
    ) -> io::Result<()> {
        self.write_frame(transmission.data_for_transmit(ack_num)?)
            .await?;
        transmission.mark_sent();
        Ok(())
    }

    /// Write an `ASHv2` frame into the buffer.
    ///
    /// # Errors
    ///
    /// Returns an error if the write operation failed or the frame buffer overflowed.
    pub async fn write_frame<F>(&mut self, frame: F) -> io::Result<()>
    where
        F: IntoIterator<Item = u8> + Display,
    {
        debug!("Writing frame: {frame}");
        self.frame.clear();
        self.frame.extend(frame);
        self.frame.stuff()?;
        self.frame
            .push(ControlByte::Flag.into())
            .map_err(|byte| Error::other(format!("Frame buffer overflow: {byte:#04X}")))?;
        timeout(T_RX_ACK_MAX, async {
            self.inner.write_all(&self.frame).await?;
            self.inner.flush().await
        })
        .await?
    }
}
