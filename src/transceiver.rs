mod retransmit;
mod rw_frame;
mod state;

use crate::ash_read::AshRead;
use crate::ash_write::AshWrite;
use crate::packet::{Data, Packet, Rst};
use crate::FrameBuffer;
use log::{debug, error, warn};
use retransmit::Retransmit;
use serialport::TTYPort;
use state::State;
use std::collections::VecDeque;
use std::io::Error;

const ACK_TIMEOUTS: usize = 4;

type Chunk = heapless::Vec<u8, { Data::MAX_PAYLOAD_SIZE }>;

#[derive(Debug)]
pub struct Transceiver {
    serial_port: TTYPort,
    state: State,
    frame_buffer: FrameBuffer,
    chunks_to_send: VecDeque<Chunk>,
    retransmits: heapless::Deque<Retransmit, ACK_TIMEOUTS>,
    frame_number: u8,
}

impl Transceiver {
    #[must_use]
    pub const fn new(serial_port: TTYPort) -> Self {
        Self {
            serial_port,
            state: State::Disconnected,
            frame_buffer: FrameBuffer::new(),
            chunks_to_send: VecDeque::new(),
            retransmits: heapless::Deque::new(),
            frame_number: 0,
        }
    }

    pub fn run(mut self) {
        loop {
            if let Err(error) = self.main() {
                error!("I/O error: {error}");
            }
        }
    }

    fn main(&mut self) -> std::io::Result<()> {
        match self.state {
            State::Disconnected | State::Failed => self.connect(),
            State::Connected => self.transceive(),
        }
    }
}

/// Establishing ASHv2 connection.
impl Transceiver {
    pub fn connect(&mut self) -> std::io::Result<()> {
        loop {
            self.send_rst()?;

            if let Packet::RstAck(rst_ack) = self
                .serial_port
                .read_packet_buffered(&mut self.frame_buffer)?
            {
                debug!("Received RSTACK: {rst_ack}");
                self.state = State::Connected;
                return Ok(());
            }
        }
    }
}

/// Send and receive packets.
impl Transceiver {
    pub fn transceive(&mut self) -> std::io::Result<()> {
        todo!("Implement transceive")
    }
}

/// Sending chunks.
impl Transceiver {
    fn send_chunks(&mut self) -> std::io::Result<()> {
        while !self.retransmits.is_full() {
            if !self.send_chunk()? {
                return Ok(());
            }
        }

        Ok(())
    }

    fn send_chunk(&mut self) -> std::io::Result<bool> {
        if let Some(chunk) = self.chunks_to_send.pop_front() {
            let frame_number = self.next_frame_number();
            self.send_data(Data::create(frame_number, chunk))?;
            Ok(true)
        } else {
            Ok(false)
        }
    }
}

/// Sending packets.
impl Transceiver {
    fn send_data(&mut self, data: Data) -> std::io::Result<()> {
        self.serial_port
            .write_frame_buffered(&data, &mut self.frame_buffer)?;
        self.enqueue_retransmit(data)?;
        Ok(())
    }

    fn send_rst(&mut self) -> std::io::Result<()> {
        self.serial_port
            .write_frame_buffered(&Rst::new(), &mut self.frame_buffer)
    }
}

/// Retransmitting packets.
impl Transceiver {
    fn enqueue_retransmit(&mut self, data: Data) -> std::io::Result<()> {
        self.retransmits.push_front(data.into()).map_err(|_| {
            Error::new(
                std::io::ErrorKind::OutOfMemory,
                "ASHv2: failed to enqueue retransmit",
            )
        })
    }

    fn retransmit(&mut self) -> std::io::Result<()> {
        for _ in 0..self.retransmits.len() {
            if let Some(retransmit) = self.retransmits.pop_back() {
                if retransmit.is_timed_out() {
                    let data = retransmit.into_data();
                    warn!("Retransmitting {:?}", data);
                    return self.send_data(data);
                }
            }
        }

        Ok(())
    }
}

/// Miscellaneous methods.
impl Transceiver {
    fn next_frame_number(&mut self) -> u8 {
        let frame_number = self.frame_number;
        self.frame_number = self.frame_number.wrapping_add(1) % 8;
        frame_number
    }
}
