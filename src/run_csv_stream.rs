use std::sync::Arc;

use futures::StreamExt;
use tracing::warn;

use crate::csv::create_transaction_stream;
use crate::domain::Ledger;

pub async fn run<R>(reader: R, ledger: Arc<Ledger>)
where
    R: tokio::io::AsyncRead + Unpin + Send + 'static,
{
    let mut transaction_stream = create_transaction_stream(reader).await;

    while let Some(transaction_result) = transaction_stream.next().await {
        match transaction_result {
            Ok(transaction) => {
                let tx = transaction.get_transaction_id();
                let client = transaction.get_client_id();
                let ledger = ledger.clone();
                // Spawn a different taks to simulate access to ledger from a differnt thread
                // but still .await it so we have deterministic results for the synchronous test
                // coming form stdin.
                let result =
                    tokio::task::spawn(async move { ledger.process_transaction(transaction) })
                        .await;

                match result {
                    Ok(ledger_result) => {
                        if let Err(e) = ledger_result {
                            warn!(client, tx, "Error processing transaction: {e}")
                        }
                    }
                    Err(e) => {
                        warn!("Join error: {e}");
                    }
                }
            }
            Err(e) => warn!(?e, "Error in transaction stream"),
        }
    }
}
