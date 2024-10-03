//! Hacks for quirks of the `ASHv2` protocol to hopefully make it a bit more stable.

use crate::transceiver::buffers::Buffers;

/// Quirks for the transceiver buffers.
impl Buffers {
    /// Checks whether the given payload is a duplicate-sent `EZSP` invalid command response.
    pub(in crate::transceiver) fn is_duplicate_invalid_command(&self, payload: &[u8]) -> bool {
        Self::is_invalid_command(payload) && (payload == self.response)
    }

    /// Checks whether the given payload is an invalid command response.
    ///
    /// The invalid command response has the structure: `[0xAA, 0x??, 0x??, 0x58]`
    /// Mostly seen in the wild as: `[0xAA, 0x80, 0x01, 0x58]`
    const fn is_invalid_command(payload: &[u8]) -> bool {
        if payload.len() != 4 {
            return false;
        }

        payload[0] == 0xAA && payload[3] == 0x58
    }
}
