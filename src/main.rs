use std::{collections::HashMap, path::PathBuf};

use anyhow::{anyhow, Context, Result};
use clap::Parser;
use csv::Reader;
use rust_decimal::prelude::*;
use serde::Deserialize;

use coding_test::Account;

/// A transaction type represented by the CSV field `type`
#[derive(Deserialize)]
#[serde(rename_all = "lowercase")]
enum CsvTransactionType {
    Deposit,
    Withdrawal,
    Dispute,
    Resolve,
    Chargeback,
}

/// A transaction represented by a CSV row
#[derive(Deserialize)]
struct CsvTransaction {
    #[serde(rename = "type")]
    tx_type: CsvTransactionType,
    client: u16,
    #[serde(rename = "tx")]
    txid: u32,
    amount: Option<Decimal>,
}

/// Transaction payments engine
#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    /// Transactions filename
    #[clap(value_parser, value_name = "TRANSACTIONS_FILE", value_hint = clap::ValueHint::FilePath)]
    transactions_filename: PathBuf,
}

fn main() -> Result<()> {
    let args = Args::parse();

    // create csv reader
    let mut reader = Reader::from_path(&args.transactions_filename).with_context(|| {
        format!(
            "Failed to read transactions from {}",
            args.transactions_filename.display()
        )
    })?;

    let mut client_accounts = HashMap::new();

    // process transactions
    for transaction in reader.deserialize() {
        let transaction: CsvTransaction =
            transaction.with_context(|| format!("Failed to parse transaction from CSV row"))?;
        // insert client account if it doesn't already exist
        let account = client_accounts
            .entry(transaction.client)
            .or_insert(Account::new(transaction.client));
        // process transaction
        use CsvTransactionType::*;
        match transaction.tx_type {
            Deposit => account.deposit(
                transaction.txid,
                transaction
                    .amount
                    .ok_or(anyhow!("amount is required for deposit transactions"))?,
            ),
            Withdrawal => account.withdraw(
                transaction
                    .amount
                    .ok_or(anyhow!("amount is required for withdrawal transactions"))?,
            ),
            Dispute => account.dispute(transaction.txid),
            Resolve => account.resolve(transaction.txid),
            Chargeback => account.chargeback(transaction.txid),
        };
    }

    // print client accounts
    println!("client,available,held,total,locked");
    for (_, account) in client_accounts.iter() {
        println!("{}", account);
    }

    Ok(())
}
