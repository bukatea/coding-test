//! # Accounts
//!
//! `accounts` is a library for describing and managing user accounts.

use rust_decimal::Decimal;

use serde::{Deserialize, Serialize};

/// Client's ID wrapper type
#[derive(Clone, Copy, PartialEq, Eq, Hash, Deserialize, Serialize)]
pub struct ClientId(pub u16);

/// Transaction's ID wrapper type
#[derive(Clone, Copy, PartialEq, Eq, Hash, Deserialize, Serialize)]
pub struct Txid(pub u32);

impl std::fmt::Display for Txid {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Transaction type
#[derive(Clone, Copy)]
pub enum TransactionType {
    Deposit(Decimal),
    Withdrawal(Decimal),
    Dispute,
    Resolve,
    Chargeback,
}

/// Transaction on an account
#[derive(Clone, Copy)]
pub struct Transaction {
    pub tx_type: TransactionType,
    pub client_id: ClientId,
    pub txid: Txid,
}

mod account;
mod accounts_handler;

pub(crate) use account::Account;

pub use accounts_handler::AccountsHandler;
