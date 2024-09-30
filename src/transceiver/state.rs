#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub enum State {
    #[default]
    Disconnected,
    Connected,
    Failed,
}
