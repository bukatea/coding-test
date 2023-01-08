//! # Account
//!
//! `account` is a module providing functionality for operating on a single account.

use rust_decimal::Decimal;
use std::collections::HashMap;

use serde::Serialize;

use crate::{ClientId, Transaction, TransactionType, Txid};

/// Deposit record
pub(crate) struct DepositRecord {
    /// Deposit amount
    amount: Decimal,
    /// Disputed
    disputed: bool,
}

/// Serializable snapshot of the client's account
#[derive(Serialize)]
pub struct AccountSnapshot {
    /// Client's ID
    #[serde(rename = "client")]
    id: ClientId,
    /// Available balance
    available: Decimal,
    /// Held balance
    held: Decimal,
    /// Total balance
    total: Decimal,
    /// Locked status
    locked: bool,
}

/// Client's account
pub(crate) struct Account {
    /// Client's ID
    id: ClientId,
    /// Record of deposits as a map of transaction IDs to deposit records
    deposits: HashMap<Txid, DepositRecord>,
    /// Available balance
    available: Decimal,
    /// Held balance
    held: Decimal,
    /// Locked status
    locked: bool,
}

impl Account {
    /// Create a new empty account
    pub(crate) fn new(id: ClientId) -> Self {
        Self {
            id,
            deposits: HashMap::new(),
            available: Decimal::ZERO,
            held: Decimal::ZERO,
            locked: false,
        }
    }

    /// Process a transaction
    pub(crate) fn process(&mut self, tx: Transaction) {
        // reject if client id does not match
        if tx.client_id != self.id {
            return;
        }
        use TransactionType::*;
        match tx.tx_type {
            Deposit(amount) => self.deposit(tx.txid, amount),
            Withdrawal(amount) => self.withdraw(amount),
            Dispute => self.dispute(tx.txid),
            Resolve => self.resolve(tx.txid),
            Chargeback => self.chargeback(tx.txid),
        }
    }

    /// Deposit funds into the account
    fn deposit(&mut self, txid: Txid, amount: Decimal) {
        // if negative amount, ignore
        if amount.is_sign_negative() {
            return;
        }
        // record deposit
        self.deposits.insert(
            txid,
            DepositRecord {
                amount,
                disputed: false,
            },
        );
        self.available += amount;
    }

    /// Withdraw funds from the account
    fn withdraw(&mut self, amount: Decimal) {
        // if locked, disallow withdrawals
        if self.locked {
            return;
        }
        // if negative amount, ignore
        if amount.is_sign_negative() {
            return;
        }
        let new_balance = self.available - amount;
        // if insufficient funds, ignore
        if new_balance.is_sign_negative() {
            return;
        }
        self.available = new_balance;
    }

    /// Dispute a transaction
    fn dispute(&mut self, txid: Txid) {
        // only deposits can be disputed
        if let Some(DepositRecord { amount, disputed }) = self.deposits.get_mut(&txid) {
            // if already disputed or if available balance cannot take a dispute, ignore
            if !*disputed && self.available >= *amount {
                // hold funds
                *disputed = true;
                self.available -= *amount;
                self.held += *amount;
            }
        }
    }

    /// Resolve a transaction
    fn resolve(&mut self, txid: Txid) {
        // only disputed deposits can be resolved
        if let Some(DepositRecord { amount, disputed }) = self.deposits.get_mut(&txid) {
            // if not disputed, ignore
            if *disputed {
                // release funds
                *disputed = false;
                self.available += *amount;
                self.held -= *amount;
            }
        }
    }

    /// Chargeback a transaction
    fn chargeback(&mut self, txid: Txid) {
        // only disputed deposits can be chargebacked
        if let Some(DepositRecord { amount, disputed }) = self.deposits.get_mut(&txid) {
            // if not disputed, ignore
            if *disputed {
                // reverse transaction and lock account
                *disputed = false;
                self.held -= *amount;
                self.locked = true;
            }
        }
    }

    /// Get a snapshot of the account
    pub(crate) fn snapshot(&self) -> AccountSnapshot {
        AccountSnapshot {
            id: self.id,
            available: self.available,
            held: self.held,
            total: self.available + self.held,
            locked: self.locked,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    #[test]
    fn deposit_positive_works() {
        let mut account = Account::new(ClientId(1));
        account.deposit(Txid(1), dec!(1.00));
        assert_eq!(account.available, dec!(1.00));
        assert_eq!(account.held, dec!(0));
    }

    #[test]
    fn deposit_negative_ignored() {
        let mut account = Account::new(ClientId(1));
        account.deposit(Txid(1), dec!(-1.00));
        assert_eq!(account.available, dec!(0));
        assert_eq!(account.held, dec!(0));
    }

    #[test]
    fn withdraw_positive_works() {
        let mut account = Account::new(ClientId(1));
        account.deposit(Txid(1), dec!(1.00));
        account.withdraw(dec!(0.50));
        assert_eq!(account.available, dec!(0.50));
        assert_eq!(account.held, dec!(0));
    }

    #[test]
    fn withdraw_negative_ignored() {
        let mut account = Account::new(ClientId(1));
        account.deposit(Txid(1), dec!(1.00));
        account.withdraw(dec!(-0.50));
        assert_eq!(account.available, dec!(1.00));
        assert_eq!(account.held, dec!(0));
    }

    #[test]
    fn withdraw_from_locked_ignored() {
        let mut account = Account::new(ClientId(1));
        account.deposit(Txid(1), dec!(1.00));
        account.locked = true;
        account.withdraw(dec!(0.50));
        assert_eq!(account.available, dec!(1.00));
        assert_eq!(account.held, dec!(0));
    }

    #[test]
    fn withdraw_insufficient_funds_ignored() {
        let mut account = Account::new(ClientId(1));
        account.deposit(Txid(1), dec!(1.00));
        account.withdraw(dec!(1.50));
        assert_eq!(account.available, dec!(1.00));
        assert_eq!(account.held, dec!(0));
    }

    #[test]
    fn dispute_deposit_works() {
        let mut account = Account::new(ClientId(1));
        let txid = Txid(1);
        account.deposit(txid, dec!(1.00));
        account.dispute(txid);
        assert_eq!(account.available, dec!(0));
        assert_eq!(account.held, dec!(1.00));
    }

    #[test]
    fn dispute_nondeposit_ignores() {
        let mut account = Account::new(ClientId(1));
        account.deposit(Txid(1), dec!(1.00));
        account.dispute(Txid(2));
        assert_eq!(account.available, dec!(1.00));
        assert_eq!(account.held, dec!(0));
    }

    #[test]
    fn resolve_deposit_works() {
        let mut account = Account::new(ClientId(1));
        let txid = Txid(1);
        account.deposit(txid, dec!(1.00));
        account.dispute(txid);
        account.resolve(txid);
        assert_eq!(account.available, dec!(1.00));
        assert_eq!(account.held, dec!(0));
    }

    #[test]
    fn resolve_undisputed_ignores() {
        let mut account = Account::new(ClientId(1));
        let txid = Txid(1);
        account.deposit(txid, dec!(1.00));
        account.resolve(txid);
        assert_eq!(account.available, dec!(1.00));
        assert_eq!(account.held, dec!(0));
    }

    #[test]
    fn chargeback_deposit_works() {
        let mut account = Account::new(ClientId(1));
        let txid = Txid(1);
        account.deposit(txid, dec!(1.00));
        account.dispute(txid);
        account.chargeback(txid);
        assert_eq!(account.available, dec!(0));
        assert_eq!(account.held, dec!(0));
        assert_eq!(account.locked, true);
    }

    #[test]
    fn chargeback_undisputed_ignores() {
        let mut account = Account::new(ClientId(1));
        let txid = Txid(1);
        account.deposit(txid, dec!(1.00));
        account.chargeback(txid);
        assert_eq!(account.available, dec!(1.00));
        assert_eq!(account.held, dec!(0));
        assert_eq!(account.locked, false);
    }

    #[test]
    fn dispute_more_than_balance_ignores() {
        let mut account = Account::new(ClientId(1));
        let txid = Txid(1);
        account.deposit(txid, dec!(1.00));
        account.withdraw(dec!(1.00));
        account.dispute(txid);
        assert_eq!(account.available, dec!(0));
        assert_eq!(account.held, dec!(0));
    }
}
