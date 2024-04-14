use futures::StreamExt;

use super::{TransactionRecord, TransactionRecordError};
use crate::domain::Transaction;

pub async fn create_transaction_stream<R>(
    reader: R,
) -> impl futures::Stream<Item = Result<Transaction, TransactionRecordError>>
where
    R: tokio::io::AsyncRead + Unpin + Send + 'static,
{
    csv_async::AsyncReaderBuilder::new()
        // trim whitespaces if we encounter them
        .trim(csv_async::Trim::All)
        // to omit the last comma for dispute|resolve|chargeback lines
        .flexible(true)
        .create_deserializer(reader)
        .into_deserialize::<TransactionRecord>()
        .map(|r| match r {
            Ok(r) => r.try_into(),
            Err(e) => Err(e.into()),
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::Decimal;

    #[tokio::test]
    async fn test_transaction_stream_works_without_spaces() {
        let test_data = "type, client,tx,amount
deposit,1,1,1.0
withdrawal,1,2,1.5
dispute,1,1,";
        let mut transaction_stream = create_transaction_stream(test_data.as_bytes()).await;

        assert_eq!(
            transaction_stream.next().await.unwrap().unwrap(),
            Transaction::Deposit {
                client: 1,
                tx: 1,
                amount: Decimal::new(1, 0),
            }
        );
        assert_eq!(
            transaction_stream.next().await.unwrap().unwrap(),
            Transaction::Withdrawal {
                client: 1,
                tx: 2,
                amount: Decimal::new(15, 1)
            }
        );
        assert_eq!(
            transaction_stream.next().await.unwrap().unwrap(),
            Transaction::Dispute { client: 1, tx: 1 }
        );
    }

    #[tokio::test]
    async fn test_transaction_stream_works_with_whitespaces() {
        let test_data = "
            type, client,tx,amount
            deposit, 1,     1,   1.0
            withdrawal  ,1, 2, 1.5
                dispute, 1, 1,
        ";
        let mut transaction_stream = create_transaction_stream(test_data.as_bytes()).await;

        assert_eq!(
            transaction_stream.next().await.unwrap().unwrap(),
            Transaction::Deposit {
                client: 1,
                tx: 1,
                amount: Decimal::new(1, 0),
            }
        );
        assert_eq!(
            transaction_stream.next().await.unwrap().unwrap(),
            Transaction::Withdrawal {
                client: 1,
                tx: 2,
                amount: Decimal::new(15, 1)
            }
        );
        assert_eq!(
            transaction_stream.next().await.unwrap().unwrap(),
            Transaction::Dispute { client: 1, tx: 1 }
        );
    }

    #[tokio::test]
    async fn test_transaction_stream_works_with_and_without_comma() {
        let test_data = "
            type, client,tx,amount
            resolve,1,2
            dispute,1,1,
        ";
        let mut transaction_stream = create_transaction_stream(test_data.as_bytes()).await;

        assert_eq!(
            transaction_stream.next().await.unwrap().unwrap(),
            Transaction::Resolve { client: 1, tx: 2 }
        );
        assert_eq!(
            transaction_stream.next().await.unwrap().unwrap(),
            Transaction::Dispute { client: 1, tx: 1 }
        );
    }

    #[tokio::test]
    async fn test_transaction_stream_returns_err_for_inexistent_transaction_type() {
        let test_data = "
            type, client,tx,amount
            inexistent,1,2
            dispute,1,1,
        ";
        let mut transaction_stream = create_transaction_stream(test_data.as_bytes()).await;

        assert!(transaction_stream.next().await.unwrap().is_err());
        assert_eq!(
            transaction_stream.next().await.unwrap().unwrap(),
            Transaction::Dispute { client: 1, tx: 1 }
        );
    }

    #[tokio::test]
    async fn test_transaction_stream_returns_err_for_deposit_or_withdrawal_without_amount() {
        let test_data = "
            type, client,tx,amount
            dispute,1,1,
            deposit,1,2
            withdrawal,1,2
            dispute,1,1,
        ";
        let mut transaction_stream = create_transaction_stream(test_data.as_bytes()).await;

        assert_eq!(
            transaction_stream.next().await.unwrap().unwrap(),
            Transaction::Dispute { client: 1, tx: 1 }
        );
        assert!(transaction_stream.next().await.unwrap().is_err());
        assert!(transaction_stream.next().await.unwrap().is_err());
        assert_eq!(
            transaction_stream.next().await.unwrap().unwrap(),
            Transaction::Dispute { client: 1, tx: 1 }
        );
    }

    #[tokio::test]
    async fn test_transaction_stream_works_with_all_transaction_types() {
        let test_data = "
            type, client, tx, amount
            deposit, 1, 1, 1.0
            withdrawal, 1, 2, 1.5
            dispute, 1, 1
            resolve, 1, 2
            chargeback, 2, 2
        ";
        let mut transaction_stream = create_transaction_stream(test_data.as_bytes()).await;

        assert_eq!(
            transaction_stream.next().await.unwrap().unwrap(),
            Transaction::Deposit {
                client: 1,
                tx: 1,
                amount: Decimal::new(1, 0),
            }
        );
        assert_eq!(
            transaction_stream.next().await.unwrap().unwrap(),
            Transaction::Withdrawal {
                client: 1,
                tx: 2,
                amount: Decimal::new(15, 1)
            }
        );
        assert_eq!(
            transaction_stream.next().await.unwrap().unwrap(),
            Transaction::Dispute { client: 1, tx: 1 }
        );
        assert_eq!(
            transaction_stream.next().await.unwrap().unwrap(),
            Transaction::Resolve { client: 1, tx: 2 }
        );
        assert_eq!(
            transaction_stream.next().await.unwrap().unwrap(),
            Transaction::Chargeback { client: 2, tx: 2 }
        );
    }
}
