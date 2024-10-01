mod channels;
mod retransmit;
mod rw_frame;
mod state;
mod wrapping_u3;

use crate::ash_read::AshRead;
use crate::ash_write::AshWrite;
use crate::packet::{Ack, Data, Nak, Packet, Rst};
use crate::request::Request;
use crate::util::next_three_bit_number;
use crate::FrameBuffer;
use channels::Channels;
use log::{debug, error, warn};
use retransmit::Retransmit;
use serialport::TTYPort;
use state::State;
use std::collections::VecDeque;
use std::io::{Error, ErrorKind};
use std::sync::mpsc::{Receiver, Sender};
use wrapping_u3::WrappingU3;

const ACK_TIMEOUTS: usize = 4;

type Chunk = heapless::Vec<u8, { Data::MAX_PAYLOAD_SIZE }>;

#[derive(Debug)]
pub struct Transceiver {
    serial_port: TTYPort,
    channels: Channels,
    state: State,
    frame_buffer: FrameBuffer,
    chunks_to_send: VecDeque<Chunk>,
    retransmits: heapless::Deque<Retransmit, ACK_TIMEOUTS>,
    frame_number: WrappingU3,
    last_received_frame_num: Option<WrappingU3>,
    response_buffer: Vec<u8>,
    reject: bool,
}

impl Transceiver {
    #[must_use]
    pub const fn new(
        serial_port: TTYPort,
        requests: Receiver<Request>,
        callback: Option<Sender<Box<[u8]>>>,
    ) -> Self {
        Self {
            serial_port,
            channels: Channels::new(requests, callback),
            state: State::Disconnected,
            frame_buffer: FrameBuffer::new(),
            chunks_to_send: VecDeque::new(),
            retransmits: heapless::Deque::new(),
            frame_number: WrappingU3::from_u8_lossy(0),
            last_received_frame_num: None,
            response_buffer: Vec::new(),
            reject: false,
        }
    }

    pub fn run(mut self) {
        loop {
            if let Err(error) = self.main() {
                error!("I/O error: {error}");
                self.reject = true;
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
        if self.reject {
            return self.try_clear_reject_condition();
        }

        // Try to receive ACKs.
        if self.retransmits.is_full() {
            while let Some(packet) = self.receive()? {
                self.handle(packet)?;
            }
        }

        // Try to retransmit packages.
        if self.retransmits.is_full() {
            while self.retransmit()? {}
        }

        // The retransmit queue is still full, so we bail out.
        if self.retransmits.is_full() {
            warn!("Retransmit queue still full.");
            return Ok(());
        }

        // Send chunks.
        self.send_chunks()?;

        while let Some(packet) = self.receive()? {
            self.handle(packet)?;
        }

        if self.chunks_to_send.is_empty() {
            while let Some(packet) = self.receive()? {
                self.handle(packet)?;
            }

            self.channels
                .response(Ok(self.response_buffer.clone().into_boxed_slice()))?;
        }

        Ok(())
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
    fn send_ack(&mut self, frame_num: u8) -> std::io::Result<()> {
        self.serial_port.write_frame_buffered(
            &Ack::from_ack_num(next_three_bit_number(frame_num)),
            &mut self.frame_buffer,
        )
    }

    fn send_data(&mut self, data: Data) -> std::io::Result<()> {
        self.serial_port
            .write_frame_buffered(&data, &mut self.frame_buffer)?;
        self.enqueue_retransmit(data)
            .inspect_err(|error| error!("Could not send DATA: {error}"))
    }

    fn send_nak(&mut self) -> std::io::Result<()> {
        debug!("Sending NAK: {}", self.ack_number());
        self.serial_port
            .write_frame_buffered(
                &Nak::from_ack_num(self.ack_number()),
                &mut self.frame_buffer,
            )
            .inspect_err(|error| error!("Could not send NAK: {error}"))
    }

    fn send_rst(&mut self) -> std::io::Result<()> {
        self.serial_port
            .write_frame_buffered(&Rst::new(), &mut self.frame_buffer)
            .inspect_err(|error| error!("Could not send RSTACK: {error}"))
    }
}

/// Receive packets.
impl Transceiver {
    fn receive(&mut self) -> std::io::Result<Option<Packet>> {
        match self
            .serial_port
            .read_packet_buffered(&mut self.frame_buffer)
        {
            Ok(packet) => Ok(Some(packet)),
            Err(error) => {
                if error.kind() == ErrorKind::TimedOut {
                    Ok(None)
                } else {
                    Err(error)
                }
            }
        }
    }

    fn handle(&mut self, packet: Packet) -> std::io::Result<()> {
        todo!("implement packet processing")
    }
}

/// Retransmitting packets.
impl Transceiver {
    fn enqueue_retransmit(&mut self, data: Data) -> std::io::Result<()> {
        self.retransmits.push_front(data.into()).map_err(|_| {
            Error::new(
                ErrorKind::OutOfMemory,
                "ASHv2: failed to enqueue retransmit",
            )
        })
    }

    fn retransmit(&mut self) -> std::io::Result<bool> {
        for _ in 0..self.retransmits.len() {
            if let Some(retransmit) = self.retransmits.pop_back() {
                if retransmit.is_timed_out() {
                    let data = retransmit.into_data();
                    warn!("Retransmitting {:?}", data);
                    self.send_data(data)?;
                    return Ok(true);
                }
            }
        }

        Ok(false)
    }
}

/// Error handling
impl Transceiver {
    fn try_clear_reject_condition(&mut self) -> std::io::Result<()> {
        todo!("clear reject condition")
    }
}

/// Miscellaneous methods.
impl Transceiver {
    /// Returns the ACK number to send.
    fn ack_number(&self) -> u8 {
        self.last_received_frame_num
            .map_or(0, |frame_num| (frame_num + 1).as_u8())
    }

    /// Returns the next frame number.
    fn next_frame_number(&mut self) -> u8 {
        let frame_number = self.frame_number.as_u8();
        self.frame_number += 1;
        frame_number
    }
}
