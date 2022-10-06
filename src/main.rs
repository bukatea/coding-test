use std::{collections::HashMap, convert::TryFrom, io, path::PathBuf};

use anyhow::{anyhow, Context, Result};
use clap::Parser;
use csv::ReaderBuilder;
use futures::future;
use rust_decimal::prelude::*;
use serde::Deserialize;
use tokio::task::JoinError;

use coding_test::{Account, AccountHandler, Transaction};

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

impl TryFrom<CsvTransaction> for Transaction {
    type Error = anyhow::Error;

    fn try_from(tx: CsvTransaction) -> Result<Self> {
        use CsvTransactionType::*;
        match tx.tx_type {
            Deposit => {
                let amount = tx
                    .amount
                    .ok_or(anyhow!("amount is required for deposit transactions"))?;
                Ok(Transaction::Deposit(tx.txid, amount))
            }
            Withdrawal => {
                let amount = tx
                    .amount
                    .ok_or(anyhow!("amount is required for withdraw transactions"))?;
                Ok(Transaction::Withdrawal(amount))
            }
            Dispute => Ok(Transaction::Dispute(tx.txid)),
            Resolve => Ok(Transaction::Resolve(tx.txid)),
            Chargeback => Ok(Transaction::Chargeback(tx.txid)),
        }
    }
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

    // create csv reader
    let mut reader = ReaderBuilder::new()
        .flexible(true)
        .from_path(&args.transactions_filename)
        .with_context(|| {
            format!(
                "Failed to read transactions from {}",
                args.transactions_filename.display()
            )
        })?;

    let mut account_handler_futures = vec![];
    let mut client_account_handlers = HashMap::new();

    // process transactions
    for transaction in reader.deserialize() {
        let transaction: CsvTransaction =
            transaction.with_context(|| "Failed to parse transaction from CSV row")?;
        // insert client account if it doesn't already exist
        let account_handler = client_account_handlers
            .entry(transaction.client)
            .or_insert_with(|| {
                let account = Account::new(transaction.client);
                let (account_handler, fut) = AccountHandler::new(account);
                account_handler_futures.push(fut);
                account_handler
            });
        // process transaction
        // unwrapped because we know that processing has not ended
        account_handler
            .process(Transaction::try_from(transaction)?)
            .with_context(|| "Failed to process transaction")
            .unwrap()?;
    }

    // end processing of transactions
    for account in client_account_handlers.values_mut() {
        account.end_processing();
    }

    // wait for account handlers to finish processing
    future::join_all(account_handler_futures)
        .await
        .into_iter()
        .collect::<std::result::Result<Vec<_>, JoinError>>()?;

    // serialize client accounts as csv
    let mut writer = csv::Writer::from_writer(io::stdout());
    for account in client_account_handlers.values() {
        writer.serialize(account.snapshot())?;
    }
    writer.flush()?;

    Ok(())
}
