//! Test `ASHv2` connection.

use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use std::thread::spawn;

use ashv2::{make_pair, open, BaudRate, HexSlice};
use clap::Parser;
use futures::SinkExt;
use log::{error, info};
use serialport::{FlowControl, SerialPort};
use tokio_stream::StreamExt;
use tokio_util::codec::Framed;

use commands::COMMANDS;
use raw_codec::RawCodec;

mod commands;
mod raw_codec;

/// Contrived maximum frame size.
const MAX_FRAME_SIZE: usize = 512;
const CHANNEL_SIZE: usize = 4;

#[derive(Debug, Parser)]
struct Args {
    #[arg(index = 1)]
    tty: String,
    #[arg(short, long, help = "keep listening for callbacks")]
    keep_listening: bool,
}

#[tokio::main]
async fn main() {
    env_logger::init();
    let args = Args::parse();

    match open(args.tty, BaudRate::RstCts, FlowControl::Software) {
        Ok(serial_port) => run(serial_port, args.keep_listening).await,
        Err(error) => error!("{error}"),
    }
}

async fn run(serial_port: impl SerialPort + 'static, keep_listening: bool) {
    let (mut ash, transceiver) = make_pair::<MAX_FRAME_SIZE, _>(serial_port, CHANNEL_SIZE, None);
    let running = Arc::new(AtomicBool::new(true));
    let transceiver_thread = spawn(|| transceiver.run(running));
    let mut framed = Framed::new(&mut ash, RawCodec);

    for (command, response) in COMMANDS {
        info!("Sending command: {:#04X}", HexSlice::new(command));

        match framed.send(command.into()).await {
            Ok(()) => {
                info!("Sent bytes: {:#04X}", HexSlice::new(command));
            }
            Err(error) => error!("Got error: {error:?}"),
        }

        if let Some(item) = framed.next().await {
            match item {
                Ok(bytes) => {
                    info!("Got response: {:#04X}", HexSlice::new(&bytes));

                    if bytes.iter().as_slice() == response {
                        info!("Response matches expected response.");
                    } else {
                        error!("Response does not match expected response.");
                    }
                }
                Err(error) => error!("Got error: {error:?}"),
            }
        }
    }

    if keep_listening {
        transceiver_thread
            .join()
            .expect("Transceiver thread panicked.");
    }
}
