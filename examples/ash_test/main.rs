//! Test `ASHv2` connection.

mod commands;

use ashv2::{open, AshFramed, BaudRate, HexSlice, Transceiver};
use clap::Parser;
use commands::COMMANDS;
use futures::SinkExt;
use log::{error, info};
use serialport::{FlowControl, SerialPort};
use std::sync::atomic::AtomicBool;
use std::sync::mpsc::sync_channel;
use std::sync::Arc;
use std::thread::spawn;
use tokio_stream::StreamExt;
use tokio_util::bytes::BytesMut;
use tokio_util::codec::{Decoder, Encoder, Framed};

#[derive(Debug, Parser)]
struct Args {
    #[arg(index = 1)]
    tty: String,
}

/// An example decoder.
#[derive(Debug)]
pub struct RawCodec;

impl Decoder for RawCodec {
    type Item = Box<[u8]>;
    type Error = std::io::Error;

    fn decode(&mut self, buffer: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        if buffer.len() >= 4 {
            Ok(Some(buffer.split().as_ref().into()))
        } else {
            Ok(None)
        }
    }
}

impl Encoder<Box<[u8]>> for RawCodec {
    type Error = std::io::Error;

    fn encode(&mut self, item: Box<[u8]>, dst: &mut BytesMut) -> Result<(), Self::Error> {
        dst.extend_from_slice(&item);
        Ok(())
    }
}

#[tokio::main]
async fn main() {
    env_logger::init();
    let args = Args::parse();

    match open(args.tty, BaudRate::RstCts, FlowControl::Software) {
        Ok(serial_port) => run(serial_port).await,
        Err(error) => error!("{error}"),
    }
}

async fn run(serial_port: impl SerialPort + 'static) {
    let (sender, receiver) = sync_channel(32);
    let transceiver = Transceiver::new(serial_port, receiver, None);
    let running = Arc::new(AtomicBool::new(true));
    let transceiver_thread = spawn(|| transceiver.run(running));
    let mut framed = Framed::new(AshFramed::<2>::new(sender), RawCodec);

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

    transceiver_thread
        .join()
        .expect("Transceiver thread panicked.");
}
