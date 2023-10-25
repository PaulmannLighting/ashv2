mod connection_handler;
mod sent_frame;
mod worker;

use super::{CANCEL, FLAG, SUBSTITUTE, TIMEOUT, X_OFF, X_ON};
use crate::Error;
use sent_frame::SentFrame;
use serialport::SerialPort;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

#[derive(Debug)]
pub struct Host<S>
where
    for<'s> S: SerialPort + 's,
{
    serial_port: Arc<Mutex<S>>,
    frames: Arc<Mutex<HashMap<u8, SentFrame>>>,
}

impl<S> Host<S>
where
    for<'s> S: SerialPort + 's,
{
    pub fn new(serial_port: S) -> Self {
        Self {
            serial_port: Arc::new(Mutex::new(serial_port)),
            frames: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn reset(&self) -> std::io::Result<Vec<u8>> {
        todo!()
    }

    pub async fn communicate(&self, payload: &[u8]) -> Result<Vec<u8>, Error> {
        todo!()
    }
}
