//! Test `ASHv2` connection.

mod commands;
mod raw_codec;

use ashv2::{open, AshFramed, BaudRate, HexSlice, Transceiver};
use clap::Parser;
use commands::COMMANDS;
use futures::SinkExt;
use log::{error, info};
use raw_codec::RawCodec;
use serialport::{FlowControl, SerialPort};
use std::sync::atomic::AtomicBool;
use std::sync::mpsc::sync_channel;
use std::sync::Arc;
use std::thread::spawn;
use tokio_stream::StreamExt;
use tokio_util::codec::Framed;

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
    let (sender, receiver) = sync_channel(32);
    let (waker_tx, waker_rx) = sync_channel(32);
    let transceiver = Transceiver::new(serial_port, receiver, waker_rx, None);
    let running = Arc::new(AtomicBool::new(true));
    let transceiver_thread = spawn(|| transceiver.run(running));
    let ash = AshFramed::<2>::new(sender, waker_tx);
    let mut framed = Framed::new(ash, RawCodec);

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
