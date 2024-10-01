use crate::transceiver::retransmit::Retransmit;
use crate::transceiver::PayloadBuffer;
use crate::FrameBuffer;

const ACK_TIMEOUTS: usize = 4;

#[derive(Debug)]
pub struct Buffers {
    pub(crate) frame: FrameBuffer,
    pub(crate) payload: PayloadBuffer,
    pub(crate) retransmits: heapless::Deque<Retransmit, ACK_TIMEOUTS>,
    pub(crate) response: Vec<u8>,
}

impl Buffers {
    /// Creates a new set of buffers.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            frame: FrameBuffer::new(),
            payload: PayloadBuffer::new(),
            retransmits: heapless::Deque::new(),
            response: Vec::new(),
        }
    }
}
