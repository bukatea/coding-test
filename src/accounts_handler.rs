//! # Accounts Handler
//!
//! `accounts_handler` is a module providing functionality for managing multiple accounts for
//! multiple clients.

use std::collections::{HashMap, HashSet};

use crate::{account::AccountSnapshot, Account, ClientId, Transaction, Txid};

/// Accounts handler for multiple clients
pub struct AccountsHandler {
    /// Map of client id to accounts
    accounts: HashMap<ClientId, Account>,
    /// Set of txids, used to ensure global uniqueness of txids
    txids: HashSet<Txid>,
}

impl AccountsHandler {
    /// Create a new accounts handler
    pub fn new() -> Self {
        Self {
            accounts: HashMap::new(),
            txids: HashSet::new(),
        }
    }

    /// Demultiplex a transaction by client id
    pub fn submit_transaction(&mut self, tx: Transaction) -> Result<(), String> {
        // ensure txid is unique
        if !self.txids.insert(tx.txid) {
            return Err(format!("duplicate txid: {}", tx.txid));
        }

        // get account for client
        let account = self
            .accounts
            .entry(tx.client_id)
            .or_insert_with(|| Account::new(tx.client_id));

        // process transaction
        Ok(account.process(tx))
    }

    /// Get snapshots of all accounts
    pub fn snapshot_accounts(&self) -> Vec<AccountSnapshot> {
        self.accounts.values().map(|a| a.snapshot()).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Transaction, TransactionType};
    use rust_decimal::Decimal;

    #[test]
    fn submit_transaction_works() {
        let mut handler = AccountsHandler::new();
        let tx = Transaction {
            tx_type: TransactionType::Deposit(Decimal::new(100, 0)),
            client_id: ClientId(1),
            txid: Txid(1),
        };
        handler.submit_transaction(tx).unwrap();
        assert_eq!(handler.accounts.len(), 1);
        assert_eq!(handler.txids.len(), 1);
    }

    #[test]
    fn duplicate_txid_fails() {
        let mut handler = AccountsHandler::new();
        let tx = Transaction {
            tx_type: TransactionType::Deposit(Decimal::new(100, 0)),
            client_id: ClientId(1),
            txid: Txid(1),
        };
        handler.submit_transaction(tx).unwrap();
        assert!(handler.submit_transaction(tx).is_err());
    }
}
