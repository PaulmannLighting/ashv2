/// `ASHv2` connection status.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub enum Status {
    #[default]
    /// No connection has been established yet.
    Disconnected,
    /// A connection has been established.
    Connected,
    /// The connection has been terminated.
    Failed,
}
