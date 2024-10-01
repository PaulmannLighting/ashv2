#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub enum Status {
    #[default]
    Disconnected,
    Connected,
    Failed,
}
