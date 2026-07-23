use ezsp::ezsp::{Error as EzspError, Status};
use ezsp::{Commands, Error, Frame, Header, Transmit};
use heapless::LenType;
use le_stream::ToLeStream;
use log::trace;

use crate::{Handle, Payload};

/// EZSP transmitter implemented directly by the `ASHv2` actor handle.
pub type Transmitter = Handle;

impl Transmit for Transmitter {
    async fn transmit(&mut self, frame: Frame<Commands>) -> Result<(), Error> {
        let (header, parameters) = frame.into();
        trace!("Sending EZSP frame: Header: {header:#04X?}, parameters: {parameters:?}");
        let mut payload = Payload::new();

        match header {
            Header::Legacy(header) => payload.try_extend(header.to_le_stream())?,
            Header::Extended(header) => payload.try_extend(header.to_le_stream())?,
        }

        payload.try_extend(parameters.to_le_stream())?;
        trace!("Sending EZSP frame (bytes): {payload:#04X?}");
        Ok(self.send(payload).await?)
    }
}

trait TryExtend<T> {
    fn try_extend<U>(&mut self, iter: U) -> Result<(), Error>
    where
        U: IntoIterator<Item = T>;
}

impl<const SIZE: usize, T, LenT> TryExtend<T> for heapless::Vec<T, SIZE, LenT>
where
    LenT: LenType,
{
    fn try_extend<I>(&mut self, iter: I) -> Result<(), Error>
    where
        I: IntoIterator<Item = T>,
    {
        for elem in iter {
            self.push(elem)
                .map_err(|_| Status::Error(EzspError::CommandTooLong))?;
        }

        Ok(())
    }
}
