use std::collections::{hash_map, HashMap};

use crate::domain::{Decimal, TransactionId};

use super::deposit_log::{DepositLog, DepositLogError};

#[derive(thiserror::Error, Debug, PartialEq)]
pub enum WalletError {
    #[error("DepositId exists")]
    DepositIdExists,
    #[error("Disputed transaction doesn't exist")]
    InexistentTransaction,
    #[error("Insufficient funds")]
    InsufficientFunds,
    #[error("DepositLog error: {0}")]
    DepositLogError(#[from] DepositLogError),
}

#[derive(Default, Debug, PartialEq)]
pub struct Wallet {
    available: Decimal,
    held: Decimal,
    locked: bool,
    deposit_log: HashMap<TransactionId, DepositLog>,
}

impl Wallet {
    pub fn deposit(&mut self, tx: TransactionId, amount: Decimal) -> Result<(), WalletError> {
        // if 'tx' exists in transaction_log don't increase balances
        if let hash_map::Entry::Vacant(transaction_map) = self.deposit_log.entry(tx) {
            transaction_map.insert(DepositLog::new(amount));
            self.available += amount;
            Ok(())
        } else {
            Err(WalletError::DepositIdExists)
        }
    }

    pub fn withdraw(&mut self, _tx: TransactionId, amount: Decimal) -> Result<(), WalletError> {
        if self.available >= amount {
            self.available -= amount;
            Ok(())
        } else {
            Err(WalletError::InsufficientFunds)
        }
    }

    pub fn dispute(&mut self, tx: TransactionId) -> Result<(), WalletError> {
        if let Some(logged_transaction) = self.deposit_log.get_mut(&tx) {
            logged_transaction.set_disputed()?;
            let disputed_amount = logged_transaction.get_amount();
            self.available -= disputed_amount;
            self.held += disputed_amount;
            Ok(())
        } else {
            Err(WalletError::InexistentTransaction)
        }
    }

    pub fn resolve(&mut self, tx: TransactionId) -> Result<(), WalletError> {
        if let Some(logged_transaction) = self.deposit_log.get_mut(&tx) {
            // .set_resolved()? returns early if status != Disputed
            logged_transaction.set_resolved()?;
            let disputed_amount = logged_transaction.get_amount();
            self.available += disputed_amount;
            self.held -= disputed_amount;
            Ok(())
        } else {
            Err(WalletError::InexistentTransaction)
        }
    }

    pub fn chargeback(&mut self, tx: TransactionId) -> Result<(), WalletError> {
        if let Some(logged_transaction) = self.deposit_log.get_mut(&tx) {
            // .set_chargedback()? returns early if status != Disputed
            logged_transaction.set_chargedback()?;
            let disputed_amount = logged_transaction.get_amount();
            self.held -= disputed_amount;
            self.locked = true;
            Ok(())
        } else {
            Err(WalletError::InexistentTransaction)
        }
    }

    pub fn get_available(&self) -> Decimal {
        self.available
    }
    pub fn get_held(&self) -> Decimal {
        self.held
    }
    pub fn get_total(&self) -> Decimal {
        self.available + self.held
    }
    pub fn get_locked_status(&self) -> bool {
        self.locked
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    #[test]
    fn test_deposit_works_with_new_transaction_id() {
        let mut wallet = Wallet::default();
        wallet.deposit(1, dec!(10)).unwrap();

        let deposit = DepositLog::new(dec!(10));

        let expected = Wallet {
            available: dec!(10),
            deposit_log: HashMap::from([(1, deposit)]),
            ..Default::default()
        };

        assert_eq!(wallet, expected);
    }

    #[test]
    fn test_deposit_fails_with_duplicate_transaction_id() {
        let mut wallet = Wallet::default();
        let deposit1 = DepositLog::new(dec!(1));
        wallet.deposit(1, dec!(1)).unwrap();

        let result = wallet.deposit(1, dec!(10));

        let expected = Wallet {
            available: dec!(1),
            deposit_log: HashMap::from([(1, deposit1)]),
            ..Default::default()
        };

        assert_eq!(result, Err(WalletError::DepositIdExists));
        assert_eq!(wallet, expected);
    }

    #[test]
    fn test_withdraw_works_with_sufficient_funds() {
        let mut wallet = Wallet::default();
        let deposit = DepositLog::new(dec!(10));
        wallet.deposit(1, dec!(10)).unwrap();

        wallet.withdraw(1, dec!(5)).unwrap();
        let expected = Wallet {
            available: dec!(5),
            deposit_log: HashMap::from([(1, deposit)]),
            ..Default::default()
        };

        assert_eq!(wallet, expected);
    }

    #[test]
    fn test_withdraw_fails_with_insufficient_funds_and_balances_remain_the_same() {
        let mut wallet = Wallet::default();
        let deposit = DepositLog::new(dec!(10));
        wallet.deposit(1, dec!(10)).unwrap();

        let result = wallet.withdraw(2, dec!(100));
        let expected = Wallet {
            available: dec!(10),
            deposit_log: HashMap::from([(1, deposit)]),
            ..Default::default()
        };
        assert_eq!(result, Err(WalletError::InsufficientFunds));
        assert_eq!(wallet, expected);
    }

    #[test]
    fn test_dispute_leaves_correct_balances_and_sets_disputed_on_deposit() {
        let mut wallet = Wallet::default();
        let deposit = DepositLog::new(dec!(10));
        wallet.deposit(1, dec!(10)).unwrap();
        let mut deposit_to_be_disputed = DepositLog::new(dec!(5));
        wallet.deposit(2, dec!(5)).unwrap();

        let expected = Wallet {
            available: dec!(15),
            deposit_log: HashMap::from([(1, deposit.clone()), (2, deposit_to_be_disputed.clone())]),
            ..Default::default()
        };
        assert_eq!(wallet, expected);

        wallet.dispute(2).unwrap();
        deposit_to_be_disputed.set_disputed().unwrap();

        let expected = Wallet {
            available: dec!(10),
            held: dec!(5),
            deposit_log: HashMap::from([(1, deposit), (2, deposit_to_be_disputed)]),
            ..Default::default()
        };

        assert_eq!(wallet, expected);
    }

    #[test]
    fn test_resolve_updates_balances_for_disputed_transaction() {
        let mut wallet = Wallet::default();
        let mut deposit = DepositLog::new(dec!(10));
        wallet.deposit(1, dec!(10)).unwrap();
        wallet.dispute(1).unwrap();
        wallet.resolve(1).unwrap();
        deposit.set_disputed().unwrap();
        deposit.set_resolved().unwrap();
        let expected = Wallet {
            available: dec!(10),
            deposit_log: HashMap::from([(1, deposit)]),
            ..Default::default()
        };
        assert_eq!(wallet, expected);
    }

    #[test]
    fn test_chargeback_updates_balances_and_freezes_account() {
        let mut wallet = Wallet::default();
        let mut deposit = DepositLog::new(dec!(10));
        wallet.deposit(1, dec!(10)).unwrap();
        wallet.dispute(1).unwrap();
        wallet.chargeback(1).unwrap();
        deposit.set_disputed().unwrap();
        deposit.set_chargedback().unwrap();
        let expected = Wallet {
            available: dec!(0),
            held: dec!(0),
            locked: true,
            deposit_log: HashMap::from([(1, deposit)]),
        };
        assert_eq!(wallet, expected);
    }
}
