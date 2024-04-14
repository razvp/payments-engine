use crate::domain::Decimal;
use crate::domain::{ClientId, TransactionId};

#[derive(Debug, PartialEq, Clone)]
pub enum Transaction {
    Deposit {
        client: ClientId,
        tx: TransactionId,
        amount: Decimal,
    },
    Withdrawal {
        client: ClientId,
        tx: TransactionId,
        amount: Decimal,
    },
    Dispute {
        client: ClientId,
        tx: TransactionId,
    },
    Resolve {
        client: ClientId,
        tx: TransactionId,
    },
    Chargeback {
        client: ClientId,
        tx: TransactionId,
    },
}

impl Transaction {
    pub fn get_transaction_id(&self) -> TransactionId {
        match self {
            Transaction::Deposit { tx, .. }
            | Transaction::Withdrawal { tx, .. }
            | Transaction::Dispute { tx, .. }
            | Transaction::Resolve { tx, .. }
            | Transaction::Chargeback { tx, .. } => *tx,
        }
    }
    pub fn get_client_id(&self) -> ClientId {
        match self {
            Transaction::Deposit { client, .. }
            | Transaction::Withdrawal { client, .. }
            | Transaction::Dispute { client, .. }
            | Transaction::Resolve { client, .. }
            | Transaction::Chargeback { client, .. } => *client,
        }
    }
}
