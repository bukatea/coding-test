use rust_decimal_macros::dec;
use std::collections::HashSet;

use coding_test::{AccountSnapshot, AccountsHandler, ClientId, Transaction, TransactionType, Txid};

#[test]
fn document_test_passes() {
    let transactions = [
        Transaction {
            tx_type: TransactionType::Deposit(dec!(1.0)),
            client_id: ClientId(1),
            txid: Txid(1),
        },
        Transaction {
            tx_type: TransactionType::Deposit(dec!(2.0)),
            client_id: ClientId(2),
            txid: Txid(2),
        },
        Transaction {
            tx_type: TransactionType::Deposit(dec!(2.0)),
            client_id: ClientId(1),
            txid: Txid(3),
        },
        Transaction {
            tx_type: TransactionType::Withdrawal(dec!(1.5)),
            client_id: ClientId(1),
            txid: Txid(4),
        },
        Transaction {
            tx_type: TransactionType::Withdrawal(dec!(3.0)),
            client_id: ClientId(2),
            txid: Txid(5),
        },
    ];

    let mut accounts = AccountsHandler::new();

    for transaction in transactions {
        accounts.submit_transaction(transaction).unwrap();
    }

    let snapshots = HashSet::from([
        AccountSnapshot {
            id: ClientId(1),
            available: dec!(1.5),
            held: dec!(0),
            total: dec!(1.5),
            locked: false,
        },
        AccountSnapshot {
            id: ClientId(2),
            available: dec!(2),
            held: dec!(0),
            total: dec!(2),
            locked: false,
        },
    ]);
    assert_eq!(
        accounts
            .snapshot_accounts()
            .into_iter()
            .collect::<HashSet<_>>(),
        snapshots
    );
}

#[test]
fn comprehensive_test_passes() {
    let transactions = [
        Transaction {
            tx_type: TransactionType::Deposit(dec!(1.0)),
            client_id: ClientId(1),
            txid: Txid(1),
        },
        Transaction {
            tx_type: TransactionType::Deposit(dec!(2.0)),
            client_id: ClientId(2),
            txid: Txid(2),
        },
        Transaction {
            tx_type: TransactionType::Deposit(dec!(3.0)),
            client_id: ClientId(2),
            txid: Txid(7),
        },
        Transaction {
            tx_type: TransactionType::Deposit(dec!(2.0)),
            client_id: ClientId(1),
            txid: Txid(3),
        },
        Transaction {
            tx_type: TransactionType::Deposit(dec!(5.0)),
            client_id: ClientId(3),
            txid: Txid(8),
        },
        Transaction {
            tx_type: TransactionType::Withdrawal(dec!(1.5)),
            client_id: ClientId(1),
            txid: Txid(4),
        },
        Transaction {
            tx_type: TransactionType::Deposit(dec!(3.0)),
            client_id: ClientId(1),
            txid: Txid(9),
        },
        Transaction {
            tx_type: TransactionType::Withdrawal(dec!(3.0)),
            client_id: ClientId(2),
            txid: Txid(5),
        },
        Transaction {
            tx_type: TransactionType::Dispute,
            client_id: ClientId(2),
            txid: Txid(2),
        },
        Transaction {
            tx_type: TransactionType::Resolve,
            client_id: ClientId(2),
            txid: Txid(2),
        },
        Transaction {
            tx_type: TransactionType::Dispute,
            client_id: ClientId(2),
            txid: Txid(2),
        },
        Transaction {
            tx_type: TransactionType::Chargeback,
            client_id: ClientId(1),
            txid: Txid(1),
        },
        Transaction {
            tx_type: TransactionType::Chargeback,
            client_id: ClientId(2),
            txid: Txid(2),
        },
        Transaction {
            tx_type: TransactionType::Withdrawal(dec!(1.0)),
            client_id: ClientId(2),
            txid: Txid(6),
        },
        Transaction {
            tx_type: TransactionType::Dispute,
            client_id: ClientId(3),
            txid: Txid(8),
        },
        Transaction {
            tx_type: TransactionType::Dispute,
            client_id: ClientId(1),
            txid: Txid(3),
        },
    ];

    let mut accounts = AccountsHandler::new();

    for transaction in transactions {
        let _ = accounts.submit_transaction(transaction);
    }

    let snapshots = HashSet::from([
        AccountSnapshot {
            id: ClientId(1),
            available: dec!(2.5),
            held: dec!(2.0),
            total: dec!(4.5),
            locked: false,
        },
        AccountSnapshot {
            id: ClientId(2),
            available: dec!(0),
            held: dec!(0),
            total: dec!(0),
            locked: true,
        },
        AccountSnapshot {
            id: ClientId(3),
            available: dec!(0),
            held: dec!(5.0),
            total: dec!(5.0),
            locked: false,
        },
    ]);
    assert_eq!(
        accounts
            .snapshot_accounts()
            .into_iter()
            .collect::<HashSet<_>>(),
        snapshots
    );
}
