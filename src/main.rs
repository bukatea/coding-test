use std::{io, path::PathBuf};

use anyhow::{anyhow, Context, Result};
use clap::Parser;
use csv::ReaderBuilder;
use rust_decimal::Decimal;

use serde::Deserialize;

use coding_test::{AccountsHandler, ClientId, Transaction, TransactionType, Txid};

/// Transaction type represented by the CSV field `type`
#[derive(Deserialize)]
#[serde(rename_all = "lowercase")]
enum CsvTransactionType {
    Deposit,
    Withdrawal,
    Dispute,
    Resolve,
    Chargeback,
}

/// Transaction represented by a CSV row
#[derive(Deserialize)]
struct CsvTransaction {
    #[serde(rename = "type")]
    tx_type: CsvTransactionType,
    client: u16,
    #[serde(rename = "tx")]
    txid: u32,
    amount: Option<Decimal>,
}

// This abstraction of the two separate transaction types is necessary because of a limitation of
// `csv::Deserialize` which does not allow to deserialize a field into an enum with heterogenous
// variants. The best that can be done is
// https://stackoverflow.com/questions/69417454/serialize-deserialize-csv-with-nested-enum-struct-with-serde-in-rust
// but this only works for serializing and not for deserializing, as documented here:
// github.com/BurntSushi/rust-csv/issues/211
// Thus, we have to deserialize into a separate struct for each variant and then convert it into the
// desired enum variant. This also allows us to do some basic validation of the CSV data, including
// checking decimal precision of `amount` to 4 places.
impl TryFrom<CsvTransaction> for Transaction {
    type Error = anyhow::Error;

    fn try_from(tx: CsvTransaction) -> Result<Self> {
        use CsvTransactionType::*;
        let amount = match tx.tx_type {
            Deposit | Withdrawal => {
                let amount = tx.amount.ok_or_else(|| {
                    anyhow!("amount is required for deposit/withdraw transactions")
                })?;
                if amount.scale() > 4 {
                    return Err(anyhow!("amount has more than 4 decimal places"));
                }
                amount
            }
            Dispute | Resolve | Chargeback => Decimal::ZERO,
        };
        let tx_type = match tx.tx_type {
            Deposit => TransactionType::Deposit(amount),
            Withdrawal => TransactionType::Withdrawal(amount),
            Dispute => TransactionType::Dispute,
            Resolve => TransactionType::Resolve,
            Chargeback => TransactionType::Chargeback,
        };

        Ok(Self {
            tx_type,
            client_id: ClientId(tx.client),
            txid: Txid(tx.txid),
        })
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

fn main() -> Result<()> {
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

    let mut accounts = AccountsHandler::new();

    // process transactions
    for transaction in reader.deserialize() {
        let transaction: CsvTransaction =
            transaction.with_context(|| "Failed to parse transaction from CSV row")?;
        // process transaction
        // ignore validation errors, including precision and missing amount
        if let Ok(transaction) = Transaction::try_from(transaction) {
            // ignore duplicate txid error
            let _ = accounts.submit_transaction(transaction);
        }
    }

    // serialize client accounts as csv
    let mut writer = csv::Writer::from_writer(io::stdout());
    for snapshot in accounts.snapshot_accounts() {
        writer.serialize(snapshot)?;
    }
    writer.flush()?;

    Ok(())
}
