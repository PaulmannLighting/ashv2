/// Futures returned by [`crate::start`] to drive the asynchronous `ASHv2` actor.
pub struct Futures<T, R> {
    /// Future that drives outbound `ASHv2` frame transmission.
    pub transmitter: T,

    /// Future that drives inbound `ASHv2` frame reception.
    pub receiver: R,
}

impl<T, R> Futures<T, R> {
    /// Create named actor futures.
    pub const fn new(transmitter: T, receiver: R) -> Self {
        Self {
            transmitter,
            receiver,
        }
    }
}
