use std::{path::PathBuf, sync::Arc};

use anyhow::{Context, Result};
use clap::Parser;
use csv_async::AsyncDeserializer;
use dashmap::DashMap;
use futures::TryStreamExt;
use rust_decimal::prelude::*;
use serde::Deserialize;
use tokio::fs::File;

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

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    // stream transactions file
    let transactions_file = File::open(&args.transactions_filename)
        .await
        .with_context(|| {
            format!(
                "Failed to read transactions from {}",
                args.transactions_filename.display()
            )
        })?;
    // create async csv deserializer
    let mut reader = AsyncDeserializer::from_reader(transactions_file);
    let records = reader.deserialize::<CsvTransaction>();

    let client_accounts = Arc::new(DashMap::new());

    // process transactions
    let fut = records.try_for_each_concurrent(None, |transaction| {
        let client_accounts = client_accounts.clone();
        async move {
            // insert client account if it doesn't already exist
            let mut account = client_accounts
                .entry(transaction.client)
                .or_insert(Account::new(transaction.client));
            // process transaction
            use CsvTransactionType::*;
            match transaction.tx_type {
                Deposit => account.deposit(
                    transaction.txid,
                    transaction
                        .amount
                        .expect("amount is required for deposit transactions"),
                ),
                Withdrawal => account.withdraw(
                    transaction
                        .amount
                        .expect("amount is required for withdrawal transactions"),
                ),
                Dispute => account.dispute(transaction.txid),
                Resolve => account.resolve(transaction.txid),
                Chargeback => account.chargeback(transaction.txid),
            };

            Ok(())
        }
    });
    fut.await?;

    // print client accounts
    println!("client,available,held,total,locked");
    for account in client_accounts.iter() {
        println!("{}", *account);
    }

    Ok(())
}
