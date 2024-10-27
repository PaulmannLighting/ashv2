#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub enum Request {
    Data(Box<[u8]>),
    Shutdown,
}
