//! Test ASHv2 connection.

use ashv2::{open, BaudRate, Host, Transceiver};
use log::{error, info};
use serialport::FlowControl;
use std::sync::mpsc::channel;
use std::thread::spawn;

const SERIAL_PORT: &str = "/dev/ttymcx0";
const VERSION_COMMAND: [u8; 4] = [0x00, 0x00, 0x00, 0x02];

#[tokio::main]
async fn main() {
    let port = open(SERIAL_PORT, BaudRate::RstCts, FlowControl::Software)
        .expect("failed to open TTY port");
    let (sender, receiver) = channel();
    let transceiver = Transceiver::new(port, receiver, None);
    let _thread_handle = spawn(|| transceiver.run());
    let host = Host::new(sender);

    match host.communicate(&VERSION_COMMAND).await {
        Ok(bytes) => info!("Got response: {bytes:?}"),
        Err(error) => error!("Got error: {error:?}"),
    }
}
