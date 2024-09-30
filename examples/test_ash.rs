//! Test ASHv2 connection.

use ashv2::{open, BaudRate, Host};
use log::{error, info};
use serialport::FlowControl;

const SERIAL_PORT: &str = "/dev/ttymcx0";
const VERSION_COMMAND: [u8; 4] = [0x00, 0x00, 0x00, 0x02];

#[tokio::main]
async fn main() {
    let port = open(SERIAL_PORT, BaudRate::RstCts, FlowControl::Software)
        .expect("failed to open TTY port");
    let host = Host::new(port, None);

    match host.communicate(&VERSION_COMMAND).await {
        Ok(bytes) => info!("Got response: {bytes:?}"),
        Err(error) => error!("Got error: {error:?}"),
    }
}
