use super::{Host, SentFrame, CANCEL, FLAG, SUBSTITUTE, TIMEOUT, X_OFF, X_ON};
use crate::packet::ack::Ack;
use crate::packet::data::Data;
use crate::packet::nak::Nak;
use crate::packet::Packet;
use crate::protocol::stuffing::Stuffing;
use log::error;
use serialport::SerialPort;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;

const MAX_BUF_CAPACITY: usize = 132 * 2; // Worst case: every byte is escaped.
const T_RX_ACK_INIT: Duration = Duration::from_millis(1600);
const T_RX_ACK_MIN: Duration = Duration::from_millis(400);
const T_RX_ACK_MAX: Duration = Duration::from_millis(3200);
const T_TX_ACK_DELAY: Duration = Duration::from_millis(20);
const T_REMOTE_NOTRDY: Duration = Duration::from_millis(1000);

#[derive(Debug)]
pub struct ConnectionHandler<S>
where
    S: SerialPort,
{
    serial_port: Arc<Mutex<S>>,
    frames: Arc<Mutex<HashMap<u8, SentFrame>>>,
    result_buffer: Vec<Data>,
    byte_buffer: [u8; 1],
    t_rx_ack: Duration,
    ack_num: u8,
    rejecting: bool,
}

impl<S> ConnectionHandler<S>
where
    S: SerialPort,
{
    pub fn run(mut self) -> std::io::Result<Vec<u8>> {
        loop {
            match self.read_packet()? {
                Packet::Data(data) => self.handle_incoming_data(data)?,
                Packet::Ack(ack) => self.clear_acked_frames(ack.ack_num())?,
                Packet::Nak(nak) => self.resend(nak.ack_num())?,
                _ => todo!(),
            }
        }
    }

    fn handle_incoming_data(&mut self, data: Data) -> std::io::Result<()> {
        if data.frame_num() == self.ack_num {
            self.rejecting = false;
            self.ack_num = (self.ack_num + 1) % 8;
            self.clear_acked_frames(data.ack_num())?;
            self.result_buffer.push(data);
            self.send_ack()
        } else if data.is_retransmit() {
            self.result_buffer.push(data);
            self.send_ack()
        } else {
            self.rejecting = true;
            self.send_nak()
        }
    }

    fn clear_acked_frames(&self, ack_num: u8) -> std::io::Result<()> {
        match self.frames.lock() {
            Ok(mut frames) => {
                frames.retain(|&key, _| key >= ack_num);
                Ok(())
            }
            Err(error) => {
                error!("could not lock vec in order to push frame: {error}");
                Err(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    "could not lock vec in order to push frame",
                ))
            }
        }
    }

    fn read_packet(&mut self) -> std::io::Result<Packet> {
        Packet::try_from(self.read_frame()?.as_slice()).map_err(Into::into)
    }

    fn read_frame(&mut self) -> std::io::Result<Vec<u8>> {
        let mut buffer = Vec::new();
        let mut skip_to_next_flag = false;

        loop {
            match self.read_byte()? {
                CANCEL => {
                    buffer.clear();
                    skip_to_next_flag = false;
                }
                FLAG => {
                    if !skip_to_next_flag && !buffer.is_empty() {
                        return Ok(buffer.into_iter().unstuff().collect());
                    }

                    buffer.clear();
                    skip_to_next_flag = false;
                }
                SUBSTITUTE => {
                    buffer.clear();
                    skip_to_next_flag = true;
                }
                X_ON | X_OFF | TIMEOUT => continue,
                byte => {
                    if buffer.len() > MAX_BUF_CAPACITY {
                        buffer.clear();
                        skip_to_next_flag = true;
                    }

                    buffer.push(byte);
                }
            }
        }
    }

    fn read_byte(&mut self) -> std::io::Result<u8> {
        match self.serial_port.lock() {
            Ok(mut serial_port) => {
                serial_port.read_exact(&mut self.byte_buffer)?;
                Ok(self.byte_buffer[0])
            }
            Err(error) => {
                error!("{error}");
                Err(std::io::Error::new(
                    std::io::ErrorKind::ConnectionRefused,
                    "could not lock serial port in order to read byte",
                ))
            }
        }
    }

    // See: 5.6 DATA frame Acknowledgement timing
    fn update_t_rx_ack(&mut self, last_ack_duration: Option<Duration>) {
        self.t_rx_ack = if let Some(duration) = last_ack_duration {
            self.t_rx_ack * 7 / 8 + duration / 2
        } else {
            self.t_rx_ack * 2
        }
        .clamp(T_RX_ACK_MIN, T_RX_ACK_MAX);
    }

    fn send_ack(&mut self) -> std::io::Result<()> {
        self.send(&Ack::from(self.ack_num))
    }

    fn send_nak(&mut self) -> std::io::Result<()> {
        self.send(&Nak::from(self.ack_num))
    }

    fn send<B>(&self, bytes: B) -> std::io::Result<()>
    where
        B: IntoIterator<Item = u8>,
    {
        match self.serial_port.lock() {
            Ok(mut serial_port) => {
                for byte in bytes.stuff() {
                    serial_port.write_all(&[byte])?;
                }

                serial_port.write_all(&[FLAG])
            }
            Err(error) => {
                error!("could not lock serial port in order to send frame: {error}");
                Err(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    "could not lock serial port in order to send frame",
                ))
            }
        }
    }

    fn resend(&self, ack_num: u8) -> std::io::Result<()> {
        todo!()
    }
}

impl<S> From<&Host<S>> for ConnectionHandler<S>
where
    for<'s> S: SerialPort + 's,
{
    fn from(host: &Host<S>) -> Self {
        Self {
            serial_port: host.serial_port.clone(),
            frames: host.frames.clone(),
            result_buffer: Vec::new(),
            byte_buffer: [0],
            t_rx_ack: T_RX_ACK_INIT,
            ack_num: 0,
            rejecting: false,
        }
    }
}
