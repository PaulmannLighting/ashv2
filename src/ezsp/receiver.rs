use std::iter::once;

use ezsp::ezsp::{Error as EzspError, Status};
use ezsp::parameters::utilities::invalid_command;
use ezsp::{
    Decode, Error, Extended, Frame, Header, Legacy, LowByte, MIN_NON_LEGACY_VERSION, Parameters,
    Parsable, Receive,
};
use le_stream::FromLeStream;
use log::{debug, trace, warn};
use tokio::sync::mpsc;

use crate::Payload;

/// Receives `ASHv2` DATA payloads and decodes them as typed EZSP frames.
pub struct Receiver {
    inner: mpsc::Receiver<Payload>,
}

impl Receiver {
    /// Creates an EZSP receiver over the `ASHv2` DATA payload channel.
    #[must_use]
    pub const fn new(inner: mpsc::Receiver<Payload>) -> Self {
        Self { inner }
    }
}

impl From<mpsc::Receiver<Payload>> for Receiver {
    fn from(inner: mpsc::Receiver<Payload>) -> Self {
        Self { inner }
    }
}

impl Receive for Receiver {
    async fn receive(&mut self, negotiated_version: Option<u8>) -> Option<Frame<Parameters>> {
        loop {
            let frame = self.inner.recv().await?;

            match parse_frame(frame, negotiated_version) {
                Ok(frame) => return Some(frame),
                Err(error) => {
                    warn!("{error}");
                }
            }
        }
    }
}

fn parse_frame(frame: Payload, negotiated_version: Option<u8>) -> Result<Frame<Parameters>, Error> {
    trace!("Decoding ASHv2 frame: {frame:#04X?}");

    let mut stream = frame.into_iter();
    let header = read_header(&mut stream, negotiated_version).ok_or(Decode::TooFewBytes)?;
    trace!("Decoded header: {header}");

    if let LowByte::Response(response) = header.low_byte() {
        if response.is_truncated() {
            return Err(Status::Error(EzspError::Truncated).into());
        }

        if response.has_overflowed() {
            return Err(Status::Error(EzspError::Overflow).into());
        }
    }

    trace!("Accumulated parameters: {stream:#04X?}");

    if header.id() == invalid_command::Response::ID {
        return match invalid_command::Response::from_le_stream_exact(&mut stream) {
            Ok(v) => Err(v.into()),
            Err(le_stream::Error::StreamNotExhausted {
                next_byte,
                instance,
            }) => {
                warn!("Stream not exhausted after parsing: {instance:?}");
                let remainder: Box<[u8]> = once(next_byte).chain(stream).collect();
                debug!("Excess bytes: {remainder:#04X?}");
                Err(instance.into())
            }
            Err(le_stream::Error::UnexpectedEndOfStream) => Err(Decode::TooFewBytes.into()),
        };
    }

    match Parameters::parse_from_le_stream(header.id(), stream) {
        Ok(parameters) => {
            trace!("Decoded parameters: {parameters:?}");
            Ok(Frame::new(header, parameters))
        }
        Err(error) => Err(error.into()),
    }
}

fn read_header<T>(stream: T, negotiated_version: Option<u8>) -> Option<Header>
where
    T: Iterator<Item = u8>,
{
    if negotiated_version.is_some_and(|version| version >= MIN_NON_LEGACY_VERSION.get()) {
        Extended::from_le_stream(stream).map(Header::Extended)
    } else {
        Legacy::from_le_stream(stream).map(Header::Legacy)
    }
}
