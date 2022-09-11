//! # Accounts
//!
//! `accounts` is a library for describing and managing user accounts.

use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use futures::Future;
use rust_decimal::Decimal;
use serde::Serialize;
use tokio::{
    sync::mpsc::{self, error::SendError},
    task::JoinError,
};

/// Serializable snapshot of the client's account
#[derive(Serialize)]
pub struct AccountSnapshot {
    /// Client's ID
    #[serde(rename = "client")]
    pub id: u16,
    /// Available balance
    pub available: Decimal,
    /// Held balance
    pub held: Decimal,
    /// Total balance
    pub total: Decimal,
    /// Locked status
    pub locked: bool,
}

/// A transaction affecting a client's account
#[derive(Debug)]
pub enum Transaction {
    /// Deposit funds into the account, requires txid and amount
    Deposit(u32, Decimal),
    /// Withdraw funds from the accout, requires amount
    Withdrawal(Decimal),
    /// Dispute transaction by txid
    Dispute(u32),
    /// Resolve transaction by txid
    Resolve(u32),
    /// Chargeback transaction by txid
    Chargeback(u32),
}

/// Client's account
pub struct Account {
    /// Client's ID
    id: u16,
    /// Record of deposits as (amount, disputed)
    deposits: HashMap<u32, (Decimal, bool)>,
    /// Available balance
    available: Decimal,
    /// Held balance
    held: Decimal,
    /// Locked status
    locked: bool,
}

impl Account {
    /// Create a new empty account
    pub fn new(id: u16) -> Self {
        Self {
            id,
            deposits: HashMap::new(),
            available: Decimal::ZERO,
            held: Decimal::ZERO,
            locked: false,
        }
    }

    /// Deposit funds into the account
    pub fn deposit(&mut self, txid: u32, amount: Decimal) {
        // if negative amount, ignore
        if amount.is_sign_negative() {
            return;
        }
        // record deposit
        self.deposits.insert(txid, (amount, false));
        self.available += amount;
    }

    /// Withdraw funds from the account
    pub fn withdraw(&mut self, amount: Decimal) {
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
    pub fn dispute(&mut self, txid: u32) {
        // only deposits can be disputed
        if let Some((amount, disputed)) = self.deposits.get_mut(&txid) {
            // if already disputed, ignore
            if !*disputed {
                // hold funds
                *disputed = true;
                self.available -= *amount;
                self.held += *amount;
            }
        }
    }

    /// Resolve a transaction
    pub fn resolve(&mut self, txid: u32) {
        // only disputed deposits can be resolved
        if let Some((amount, disputed)) = self.deposits.get_mut(&txid) {
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
    pub fn chargeback(&mut self, txid: u32) {
        // only disputed deposits can be chargebacked
        if let Some((amount, disputed)) = self.deposits.get_mut(&txid) {
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
    pub fn snapshot(&self) -> AccountSnapshot {
        AccountSnapshot {
            id: self.id,
            available: self.available,
            held: self.held,
            total: self.available + self.held,
            locked: self.locked,
        }
    }
}

/// Accounts handler
pub struct AccountHandler {
    /// Thread-safe account
    account: Arc<Mutex<Account>>,
    /// Transaction transmitter,
    tx: Option<mpsc::UnboundedSender<Transaction>>,
}

impl AccountHandler {
    /// Create a new account handler
    pub fn new(account: Account) -> (Self, impl Future<Output = Result<(), JoinError>>) {
        let (tx, mut rx) = mpsc::unbounded_channel::<Transaction>();
        let account = Arc::new(Mutex::new(account));
        let account_clone = account.clone();
        let fut = tokio::spawn(async move {
            while let Some(transaction) = rx.recv().await {
                let mut account = account_clone.lock().unwrap();
                use Transaction::*;
                match transaction {
                    Deposit(txid, amount) => account.deposit(txid, amount),
                    Withdrawal(amount) => account.withdraw(amount),
                    Dispute(txid) => account.dispute(txid),
                    Resolve(txid) => account.resolve(txid),
                    Chargeback(txid) => account.chargeback(txid),
                }
            }
        });
        (
            Self {
                account,
                tx: Some(tx),
            },
            fut,
        )
    }

    /// End processing of transactions to stabilize account state
    pub fn end_processing(&mut self) {
        self.tx.take();
    }

    /// Process transasction
    pub fn process(
        &mut self,
        transaction: Transaction,
    ) -> Option<Result<(), SendError<Transaction>>> {
        self.tx.as_ref().map(|tx| tx.send(transaction))
    }

    /// Get a snapshot of the account
    pub fn snapshot(&self) -> AccountSnapshot {
        self.account.lock().unwrap().snapshot()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    #[test]
    fn deposit_positive_works() {
        let mut account = Account::new(1);
        account.deposit(1, dec!(1.00));
        assert_eq!(account.available, dec!(1.00));
        assert_eq!(account.held, dec!(0));
    }

    #[test]
    fn deposit_negative_ignored() {
        let mut account = Account::new(1);
        account.deposit(1, dec!(-1.00));
        assert_eq!(account.available, dec!(0));
        assert_eq!(account.held, dec!(0));
    }

    #[test]
    fn withdraw_positive_works() {
        let mut account = Account::new(1);
        account.deposit(1, dec!(1.00));
        account.withdraw(dec!(0.50));
        assert_eq!(account.available, dec!(0.50));
        assert_eq!(account.held, dec!(0));
    }

    #[test]
    fn withdraw_negative_ignored() {
        let mut account = Account::new(1);
        account.deposit(1, dec!(1.00));
        account.withdraw(dec!(-0.50));
        assert_eq!(account.available, dec!(1.00));
        assert_eq!(account.held, dec!(0));
    }

    #[test]
    fn withdraw_from_locked_ignored() {
        let mut account = Account::new(1);
        account.deposit(1, dec!(1.00));
        account.locked = true;
        account.withdraw(dec!(0.50));
        assert_eq!(account.available, dec!(1.00));
        assert_eq!(account.held, dec!(0));
    }

    #[test]
    fn withdraw_insufficient_funds_ignored() {
        let mut account = Account::new(1);
        account.deposit(1, dec!(1.00));
        account.withdraw(dec!(1.50));
        assert_eq!(account.available, dec!(1.00));
        assert_eq!(account.held, dec!(0));
    }

    #[test]
    fn dispute_deposit_works() {
        let mut account = Account::new(1);
        account.deposit(1, dec!(1.00));
        account.dispute(1);
        assert_eq!(account.available, dec!(0));
        assert_eq!(account.held, dec!(1.00));
    }

    #[test]
    fn dispute_nondeposit_ignores() {
        let mut account = Account::new(1);
        account.deposit(1, dec!(1.00));
        account.dispute(2);
        assert_eq!(account.available, dec!(1.00));
        assert_eq!(account.held, dec!(0));
    }

    #[test]
    fn resolve_deposit_works() {
        let mut account = Account::new(1);
        account.deposit(1, dec!(1.00));
        account.dispute(1);
        account.resolve(1);
        assert_eq!(account.available, dec!(1.00));
        assert_eq!(account.held, dec!(0));
    }

    #[test]
    fn resolve_undisputed_ignores() {
        let mut account = Account::new(1);
        account.deposit(1, dec!(1.00));
        account.resolve(1);
        assert_eq!(account.available, dec!(1.00));
        assert_eq!(account.held, dec!(0));
    }

    #[test]
    fn chargeback_deposit_works() {
        let mut account = Account::new(1);
        account.deposit(1, dec!(1.00));
        account.dispute(1);
        account.chargeback(1);
        assert_eq!(account.available, dec!(0));
        assert_eq!(account.held, dec!(0));
        assert_eq!(account.locked, true);
    }

    #[test]
    fn chargeback_undisputed_ignores() {
        let mut account = Account::new(1);
        account.deposit(1, dec!(1.00));
        account.chargeback(1);
        assert_eq!(account.available, dec!(1.00));
        assert_eq!(account.held, dec!(0));
        assert_eq!(account.locked, false);
    }
}
