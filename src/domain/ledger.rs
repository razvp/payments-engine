use std::collections::HashMap;

use parking_lot::{MappedRwLockReadGuard, Mutex, RwLock, RwLockReadGuard, RwLockWriteGuard};
use tracing::info;

use super::{ClientId, Transaction, Wallet, WalletError};

#[derive(thiserror::Error, Debug)]
pub enum LedgerError {
    #[error("Client `{0}` does not exist")]
    InexistentClient(ClientId),
    #[error("Io error: {0}")]
    IoError(#[from] std::io::Error),
    #[error("Wallet error: {0}")]
    WalletError(#[from] WalletError),
}

#[derive(Default, Debug)]
pub struct Ledger {
    clients: RwLock<HashMap<ClientId, Mutex<Wallet>>>,
}

impl Ledger {
    pub fn new() -> Ledger {
        Ledger::default()
    }

    pub fn process_transaction(&self, transaction: Transaction) -> Result<(), LedgerError> {
        info!(?transaction, "Processing");
        match transaction {
            Transaction::Deposit { client, tx, amount } => Ok(self
                // Only `Deposits` can create new clients
                .get_existing_or_create_client(&client)
                .lock()
                .deposit(tx, amount)?),
            Transaction::Withdrawal { client, tx, amount } => Ok(self
                .get_existing_client(&client)
                .ok_or(LedgerError::InexistentClient(client))?
                .lock()
                .withdraw(tx, amount)?),
            Transaction::Dispute { client, tx } => Ok(self
                .get_existing_client(&client)
                .ok_or(LedgerError::InexistentClient(client))?
                .lock()
                .dispute(tx)?),
            Transaction::Resolve { client, tx } => Ok(self
                .get_existing_client(&client)
                .ok_or(LedgerError::InexistentClient(client))?
                .lock()
                .resolve(tx)?),
            Transaction::Chargeback { client, tx } => Ok(self
                .get_existing_client(&client)
                .ok_or(LedgerError::InexistentClient(client))?
                .lock()
                .chargeback(tx)?),
        }
    }

    /// Returns a MappedRwLockReadGuard because the `Mutex<Wallet>`
    /// references the read-lock.
    ///
    /// We first try to find the client through a read-lock so other threads can also read
    /// the `Ledger`. If it doesn't exist, we need a write-lock to create the Client
    fn get_existing_or_create_client(
        &self,
        client: &ClientId,
    ) -> MappedRwLockReadGuard<Mutex<Wallet>> {
        let read_lock = self.clients.read();
        if read_lock.contains_key(client) {
            RwLockReadGuard::map(read_lock, |hm| hm.get(client).unwrap())
        } else {
            // Drop read lock to avoid deadlock
            drop(read_lock);
            // We need a write-lock to add a new client
            let mut write_lock = self.clients.write();
            // Use entry instead of insert, in case another thread created
            // the client in the time between the dropping of the read-lock
            // and aquiring the write-lock
            let _ = write_lock.entry(*client).or_default();

            // Downgrade the write-lock to a read-lock and return
            RwLockReadGuard::map(
                RwLockWriteGuard::downgrade(write_lock),
                |hm: &HashMap<ClientId, Mutex<Wallet>>| hm.get(client).unwrap(),
            )
        }
    }

    fn get_existing_client(
        &self,
        client: &ClientId,
    ) -> Option<MappedRwLockReadGuard<Mutex<Wallet>>> {
        let read_lock = self.clients.read();

        RwLockReadGuard::try_map(read_lock, |hm| hm.get(client)).ok()
    }

    pub fn dump_to_writer<W>(&self, w: &mut W) -> Result<(), LedgerError>
    where
        W: std::io::Write,
    {
        let map = self.clients.read();
        w.write_all("client, available, held, total, locked\n".as_bytes())
            .unwrap();
        for (client_id, wallet) in map.iter() {
            let wallet = wallet.lock();
            w.write_all(
                format!(
                    "{}, {}, {}, {}, {}\n",
                    client_id,
                    wallet.get_available(),
                    wallet.get_held(),
                    wallet.get_total(),
                    wallet.get_locked_status()
                )
                .as_bytes(),
            )
            .unwrap();
        }
        w.flush()?;
        Ok(())
    }
}
