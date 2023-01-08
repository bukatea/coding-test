# coding-test
simple transaction payments engine

Given a file of transactions for a bunch of clients, this app executes the transactions in order and prints out the final account state of each client in CSV format to stdout.

# Run

To run, do

```sh
cargo run -- <TRANSACTIONS_FILE>
```

You can also do

```sh
cargo run -- -h
```

for help on how to use the CLI.

## Assumptions and Interpretations of Requirements
* As disputes are required to decrease available funds and increase held funds and a chargeback involves decreasing held funds from a previous dispute (following the PDF), only deposits make sense to be disputed. This is because a chargeback is the reversing of a transaction, and since chargeback decreases funds, it is a reverse of increasing funds, which can only be done through deposit. In other words, a dispute puts a transaction into question of its legitimacy, transferring the associated funds from available to held. This only makes sense in the context of a deposit because it is taking the client's current available balance and decreasing it (by transferring it to current held balance), implying an earlier questionable increase of funds (a deposit).
* Accordingly, resolves and chargebacks will also only work in the context of disputed deposits.
* Locked accounts will be blocked from withdrawals. Crediting an account traditionally is never blocked. Source: https://www.investopedia.com/terms/f/frozenaccount.asp.
* An attempt for a negative amount for any transaction type is ignored, treating it as an error on the partner's side.
* If a dispute is attempted referencing a deposit, it will only succeed if there is sufficient balance in the available funds to move the deposited funds from available to held.
* If a dispute of a transaction is made that is then resolved, a redispute of that transaction is allowed. (If the dispute is chargebacked, a redispute will not be allowed, as the account will have been locked.)
* Transaction ids are expected to be globally unique across all accounts. If a duplicate transaction id is detected across any two accounts, the second duplicate transaction will not be executed and will be ignored.
* Input for amount for deposit and withdrawal transactions will be accepted up to and including 4 decimal places. If more than 4 decimal places is entered as account, the transaction will be ignored.
* Amounts must be specified for deposit and withdrawal transactions, as otherwise the transaction would be meaningless. If amounts cannot be detected for those transactions, the transaction is ignroed.

## Notes
* I use `rust_decimal` for no loss of precision when working with amounts and balances.
