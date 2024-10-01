use crate::transceiver::buffers::Buffers;
use crate::transceiver::channels::Channels;
use crate::transceiver::state::State;
use serialport::TTYPort;

/// Callback handler.
#[derive(Debug)]
pub struct CallbackHandler<'a> {
    serial_port: &'a mut TTYPort,
    channels: &'a mut Channels,
    buffers: &'a mut Buffers,
    state: &'a mut State,
}

impl<'a> CallbackHandler<'a> {
    /// Creates a new callback handler.
    #[must_use]
    pub fn new(
        serial_port: &'a mut TTYPort,
        channels: &'a mut Channels,
        buffers: &'a mut Buffers,
        state: &'a mut State,
    ) -> Self {
        Self {
            serial_port,
            channels,
            buffers,
            state,
        }
    }

    pub fn run(mut self) -> std::io::Result<()> {
        todo!("Implement the callback handler");
    }
}
