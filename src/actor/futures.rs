/// Futures returned by [`crate::start`] to drive the asynchronous `ASHv2` actor.
pub struct Futures<W, T, R> {
    /// Future that drives the blocking serial-port worker and returns the serial port.
    pub serial_worker: W,
    /// Future that drives outbound `ASHv2` frame transmission.
    pub transmitter: T,
    /// Future that drives inbound `ASHv2` frame reception.
    pub receiver: R,
}

impl<W, T, R> Futures<W, T, R> {
    /// Create named actor futures.
    pub const fn new(serial_worker: W, transmitter: T, receiver: R) -> Self {
        Self {
            serial_worker,
            transmitter,
            receiver,
        }
    }
}
