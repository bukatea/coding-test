# kraken-coding-test
transaction payments engine for kraken coding challenge

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
* Locked accounts will only be blocked from withdrawals. Crediting an account traditionally is never blocked.
* An attempt for a negative amount for any transaction type is ignored, treating it as an error on the partner's side.

## Notes
* I have written two branches that both solve the given problem. The `main` branch is the "naive" implementation, which simply iterates over the CSV serially and processes each transaction before moving onto the next, no concurrency or parallelism involved. On the other hand, the `async-optimization` branch is the slightly more optimized implementation that uses `tokio`'s concurrency and parallelism features to potentially process multiple transactions at once. Namely, it reads lines from the CSV serially (this must be serial because each client's transactions are written in order, dependent on their previous transactions) but after it reads a line, it sends it off to a channel that processes the transaction asynchronously (and in parallel) in a `tokio` task. This allows the app to concurrently execute/process multiple transactions regarding **different clients** at once, where all the transactions regarding the same client are processed in order of their appearance in the CSV. This is the best we can get by adding concurrency/parallelism. We cannot read lines from the CSV concurrently (using `for_each_concurrent` or `try_for_each_concurrent`) because this brings the possibility that later lines are processed before the earlier and thus lock the map before the earlier transactions, creating possibility for error.
    * Why are there two branches? Why isn't the async one just used as the main branch? Isn't it always better? I decided against having just one main branch implementing the async version because I found that for smaller transaction CSVs the serial way is simply faster than the async version because it does not have to manage any overhead for multiple threads of concurrent execution. Furthermore, as the tasks of actually updating account state are nonblocking, really fast, and do not involve async/await at all (e.g. lines 57-80 in `main` branch), there is no immediate gain from adding asynchronous code, as there is nothing to await on and so there are no points in the updating code that work can be passed to another thread. Ultimately, however, in the long run with huge transaction CSVs asynchronously processing some transactions will improve overall throughput at the cost of overhead, negligible at high transaction volumes. Furthermore, I also expected that the problem itself sort of hints that account state updating in general could cost more in a real system, and I wanted to present a scalable solution for this case as well.
    * Why do I collect account handler futures in a separate `account_handler_futures` `Vec` in the `async-optimization` branch? This is due to a small disadvantage when adding async code. Because we now process transactions not in the main thread but instead in its own per-client thread, the main thread will not know without extra work when there are no more transactions and the CSV file is finished being read. It will not know when to stop. One of the ways I thought of to get around this was to expose futures for each handler and a method to signal the end of processing of the transactions so that the `tokio` task knows to stop processing. This is what I do here. I add an `end_processing` function to `AccountHandler` that drops the sending end of the channel, which automatically causes the receiving of new transactions to end (currently processing transactions finish processing, which is what we want). Once the currently processing transactions finish, the `tokio` task will end and this will reflect in `account_handler_futures`. Thus, in `main` after I read the entire CSV and submit transactions off to their respective channels to process, I signal end of processing for new transactions, wait for currently processing transactions to finish with `join_all`, and then read the account state in a snapshot. Note however that in a real-world situation, `AccountHandler` would always be running and there wouldn't necessarily be an "end" to the transactions, so this signalling and joining wouldn't be needed in those situations.
* I use `rust_decimal` for no loss of precision when working with amounts and balances.

