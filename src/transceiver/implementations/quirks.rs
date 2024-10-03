//! Hacks for quirks of the `ASHv2` protocol to hopefully make it a bit more stable.

use crate::transceiver::buffers::Buffers;

/// This specific `EZSP` invalid command response without `reason: EzspStatus` field may be sent
/// repeatedly by the NCP if the host-sent multi-package request was utterly garbage.
const QUIRKY_INVALID_COMMAND_RESPONSE: [u8; 4] = [0xAA, 0x80, 0x01, 0x58];

/// Quirks for the transceiver buffers.
impl Buffers {
    /// Checks whether the given payload is a duplicate-sent `EZSP` invalid command response.
    pub(in crate::transceiver) fn is_duplicate_invalid_command(&self, payload: &[u8]) -> bool {
        (payload == QUIRKY_INVALID_COMMAND_RESPONSE)
            && (self.response == QUIRKY_INVALID_COMMAND_RESPONSE)
    }
}
