mod callbacks;
mod connect;
mod constants;
mod frame_io;
mod misc;
mod receive;
mod reject;
mod reset;
mod retransmits;
mod send;
mod transaction;

use crate::channels::Channels;
use crate::packet::Data;
use crate::protocol::AshChunks;
use crate::request::Request;
use crate::retransmit::Retransmit;
use crate::status::Status;
use crate::wrapping_u3::WrappingU3;
use crate::FrameBuffer;
use log::error;
use serialport::TTYPort;
use std::sync::mpsc::{Receiver, Sender};
use std::time::SystemTime;

/// ASHv2 transceiver.
///
/// The transceiver is responsible for handling the communication between the host and the NCP.
/// It is supposed to be run in a separate thread.
/// A [`Host`](crate::Host) can be used to communicate with the NCP via the transceiver.
///
/// # Usage
///
/// ```
/// use std::sync::mpsc::channel;
/// use std::thread::spawn;
/// use ashv2::{open, BaudRate, Host, Transceiver};
///
/// let serial_port = open("/dev/ttyUSB0", BaudRate::RstCts).unwrap();
/// let (sender, receiver) = channel();
/// let transceiver = Transceiver::new(serial_port, receiver, None);
/// let thread_handle = spawn(move || transceiver.run());
/// let host = Host::from(sender);
/// let response = host.communicate(&[0x00, 0x01, 0x02, 0x03]).await.unwrap();
/// println!("{response:?}");
/// ```
#[derive(Debug)]
pub struct Transceiver {
    serial_port: TTYPort,
    channels: Channels,
    // Buffers.
    frame_buffer: FrameBuffer,
    payload_buffer: heapless::Vec<u8, { Data::MAX_PAYLOAD_SIZE }>,
    retransmits: heapless::Vec<Retransmit, { Self::ACK_TIMEOUTS }>,
    response_buffer: Vec<u8>,
    // State.
    status: Status,
    last_n_rdy_transmission: Option<SystemTime>,
    frame_number: WrappingU3,
    last_received_frame_num: Option<WrappingU3>,
    reject: bool,
    within_transaction: bool,
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
            // Buffers.
            frame_buffer: heapless::Vec::new(),
            payload_buffer: heapless::Vec::new(),
            retransmits: heapless::Vec::new(),
            response_buffer: Vec::new(),
            // State.
            status: Status::Disconnected,
            last_n_rdy_transmission: None,
            frame_number: WrappingU3::from_u8_lossy(0),
            last_received_frame_num: None,
            reject: false,
            within_transaction: false,
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
        match self.status {
            Status::Disconnected | Status::Failed => self.connect(),
            Status::Connected => self.communicate(),
        }
    }

    pub fn communicate(&mut self) -> std::io::Result<()> {
        if self.reject {
            return self.try_clear_reject_condition();
        }

        if let Some(bytes) = self.channels.receive()? {
            self.transaction(bytes.ash_chunks()?)?;
        } else {
            self.handle_callbacks()?;
        }

        Ok(())
    }
}
