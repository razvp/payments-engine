use crate::domain::ClientId;
use crate::domain::Decimal;
use crate::domain::Transaction;
use crate::domain::TransactionId;

#[derive(thiserror::Error, Debug)]
pub enum TransactionRecordError {
    #[error("Missing amount field")]
    MissingAmountError,
    #[error("csv error")]
    CsvError(#[from] csv_async::Error),
}

#[derive(serde::Deserialize, Debug)]
pub struct TransactionRecord {
    r#type: TransactionType,
    client: ClientId,
    tx: TransactionId,
    amount: Option<Decimal>,
}

#[derive(serde::Deserialize, Debug, PartialEq, Clone)]
#[serde(rename_all = "lowercase")]
enum TransactionType {
    Deposit,
    Withdrawal,
    Dispute,
    Resolve,
    Chargeback,
}

impl TryFrom<TransactionRecord> for Transaction {
    type Error = TransactionRecordError;

    fn try_from(value: TransactionRecord) -> Result<Self, Self::Error> {
        let tx = value.tx;
        let client = value.client;
        match value.r#type {
            TransactionType::Deposit => {
                let amount = value
                    .amount
                    .ok_or(TransactionRecordError::MissingAmountError)?;
                Ok(Self::Deposit { client, tx, amount })
            }
            TransactionType::Withdrawal => {
                let amount = value
                    .amount
                    .ok_or(TransactionRecordError::MissingAmountError)?;
                Ok(Self::Withdrawal { client, tx, amount })
            }
            TransactionType::Dispute => Ok(Self::Dispute { client, tx }),
            TransactionType::Resolve => Ok(Self::Resolve { client, tx }),
            TransactionType::Chargeback => Ok(Self::Chargeback { client, tx }),
        }
    }
}
