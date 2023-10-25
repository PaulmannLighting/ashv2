use crate::protocol::host::worker::Transaction;

#[derive(Debug)]
pub struct ActiveTransaction {
    id: usize,
    transaction: Transaction,
}

impl ActiveTransaction {
    #[must_use]
    pub const fn new(id: usize, transaction: Transaction) -> Self {
        Self { id, transaction }
    }
}

impl From<(usize, Transaction)> for ActiveTransaction {
    fn from((id, transaction): (usize, Transaction)) -> Self {
        Self::new(id, transaction)
    }
}

impl From<ActiveTransaction> for (usize, Transaction) {
    fn from(active_transaction: ActiveTransaction) -> Self {
        (active_transaction.id, active_transaction.transaction)
    }
}
