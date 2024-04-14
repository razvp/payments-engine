use crate::domain::Decimal;

#[allow(clippy::enum_variant_names)]
#[derive(thiserror::Error, Debug, PartialEq)]
pub enum DepositLogError {
    #[error("Can't dispute transaction, only `New` transactions are disputable")]
    CantDispute,
    #[error("Can't resolve undisputed deposit")]
    CantResolveUndisputed,
    #[error("Can't chargeback undisputed deposit")]
    CantChargebackUndisputed,
}

#[derive(Debug, PartialEq, Clone)]
pub struct DepositLog {
    amount: Decimal,
    status: DepositStatus,
}

impl DepositLog {
    pub fn new(amount: Decimal) -> Self {
        Self {
            amount,
            status: DepositStatus::New,
        }
    }
}

#[derive(Debug, PartialEq, Clone)]
enum DepositStatus {
    New,
    Disputed,
    Resolved,
    Chargedback,
}

impl DepositLog {
    pub fn get_amount(&self) -> Decimal {
        self.amount
    }
    pub fn set_disputed(&mut self) -> Result<(), DepositLogError> {
        match self.status {
            DepositStatus::New => {
                self.status = DepositStatus::Disputed;
                Ok(())
            }
            _ => Err(DepositLogError::CantDispute),
        }
    }

    pub fn set_resolved(&mut self) -> Result<(), DepositLogError> {
        match self.status {
            DepositStatus::Disputed => {
                self.status = DepositStatus::Resolved;
                Ok(())
            }
            _ => Err(DepositLogError::CantResolveUndisputed),
        }
    }

    pub fn set_chargedback(&mut self) -> Result<(), DepositLogError> {
        match self.status {
            DepositStatus::Disputed => {
                self.status = DepositStatus::Chargedback;
                Ok(())
            }
            _ => Err(DepositLogError::CantChargebackUndisputed),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    #[test]
    fn test_set_resolve_fails_for_undisputed_deposit() {
        let mut deposit_log = DepositLog::new(dec!(1));
        let result = deposit_log.set_resolved();
        assert_eq!(Err(DepositLogError::CantResolveUndisputed), result);
    }

    #[test]
    fn test_set_chargeback_fails_for_undisputed_deposit() {
        let mut deposit_log = DepositLog::new(dec!(1));
        let result = deposit_log.set_chargedback();
        assert_eq!(Err(DepositLogError::CantChargebackUndisputed), result);
    }
}
