//! Reject state management.
//!
//! `ASH` sets the Reject Condition after receiving a `DATA` frame
//! with any of the following attributes:
//!
//!   * Has an incorrect CRC.
//!   * Has an invalid control byte.
//!   * Is an invalid length for the frame type.
//!   * Contains a low-level communication error (e.g., framing, overrun, or overflow).
//!   * Has an invalid ackNum.
//!   * Is out of sequence.
//!   * Was valid, but had to be discarded due to lack of memory to store it.
//!
use crate::Transceiver;
use log::trace;
use serialport::SerialPort;

impl<T> Transceiver<T>
where
    T: SerialPort,
{
    /// Enter the rejection state.
    pub(in crate::transceiver) fn enter_reject(&mut self) -> std::io::Result<()> {
        if self.state.reject {
            Ok(())
        } else {
            trace!("Entering rejection state.");
            self.state.reject = true;
            self.nak()
        }
    }

    /// Leave the rejection state.
    pub(in crate::transceiver) fn leave_reject(&mut self) {
        if self.state.reject {
            trace!("Leaving rejection state.");
            self.state.reject = false;
        }
    }
}
