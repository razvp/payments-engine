## Description
Project simulating an async payment engine with as little locking as possible. <br/>
Jump to [Requirements and assumptions](#requirements-and-assumptions) or see how to [run](#running-and-inputoutput).

## Design choices
### Async and multithreadding:
We create a `Ledger` type that can be safely accessed from multiple threads:
```rust
pub struct Ledger {
    clients: RwLock<HashMap<ClientId, Mutex<Wallet>>>,
}

pub struct Wallet {
    available: Decimal,
    held: Decimal,
    locked: bool,
    deposit_log: HashMap<TransactionId, DepositLog>,
}
```
- `Ledger` is esentially a map from `ClientId` to a corresponding `Wallet`.
- When wrapped in an `Arc`, we can use the `Ledger` from multple threads.
- This structure allows us to **process transactions in parallel** as long as they reference different clients. `Wallet` integrity is assured by the `Mutex` wrapping it, so it can't be read or mutated by 2 threads at the same time.
- Client/wallet creation (`Ledger` blocking) happens just when a client `Deposits` and doesn't already exist in the `Ledger`.
- Parallel processing of the transactions is achieved with the help of two private methods:
    - `get_existing_client()` which returns the protected wallet in an `Option` (None if it doesn't exist). This method only uses a read-lock.
    - `get_existing_or_create_client()` which returns the protected wallet. It first tries to get the wallet through a read-lock, but if the client doesn't exist it creates it with a write-lock. The same write-lock gets downgraded to a read-lock and returned.
        - We used this mechanism to be sure that the `Ledger`is blocked for as short of a period as possible.
        - The only worring case here would be if we get a Deposit for a client that doesn't exist, followed very fast by another transaction on the same client. Between the dropping of the read-lock and aquiring of the write-lock those transactions could fail. This wouldn't really be an issue in a real world system because we would also have a `create_client` API that would be called before any transaction.
        - there are some other explanations in the [code](https://github.com/razvp/payments-engine/blob/61a66e2af4caa1e32d417791862f5e6c23bdae0a/src/domain/ledger.rs#L59-L87).
- We used the `parking_lot` crate for better synchronization primitives and helpful types like `MappedRwLockReadGuard`.
### Safety and usability/maintainability:
- Used tests, especially for the critical bits.
- `rust_decimal` crate provides a fixed-precision `Decimal` type suitable for financial calculations.
- `tracing` crate provides structured logging, helpful especially in an async context. All logs/warning go to `stderr` so `stdout` will write just the results.
- `thiserror`crate helps defining Error types with less boilerplate. Errors types are created to provide good insight for tracing.
- Used `csv-async` and `serde` crates to insure input is corectly parsed (parser configured to trim whitespaces and omit trailing commas).
- Used the type system to ensure correctness. For example: a `Resolve` or `Chargeback` can only apply to `Diputed` transactions and only a `Deposit` can be `Disputed`. This also provides good maintability and ability to add features.
- Use of common Traits like `Default`, `TryInto`. Right now `Ledger::new()` maps to `Ledger::default()` . If we need to add new fields to the `Ledger` struct to acomplish other requirements we can change this without changing the API.
- Used `rustfmt` and `clippy`.
### Others:
- The project is structured in a library and also an executable. This makes the code reusable. This could be further improved by separating to creates and defining interfaces through `Traits`.
- There are some comments for tricky parts. This could be further improved through documentation and doc-tests.
## Requirements and assumptions
- The system should be async and multi-threadding capable.
- A client has a **Wallet** that keeps track of `available` and `held` amounts and `locked`status . The `total` amounts can be computed by adding `available` and `held`.
- A client's **Wallet gets created on the first Deposit**. Other transaction types referencing an inexistent client are ignored.
- **Only `Deposits` can be `Disputed`**
- Input is in CSV format. Commas can be missing and whitespaces should be ignored.
### Transaction types:
| **type**   | **client** | **tx** | **amount(optional)** |
|------------|------------|--------|----------------------|
| deposit    |      1     |    1   |        10.1234       |
| withdrawal |      1     |    2   |           8          |
| dispute    |      1     |    1   |                      |
| resolve    |      1     |    1   |                      |
| chargeback |      1     |    1   |                      |
#### Transactions description:
  1. Deposit
      - increases the `available` amount
      - fails if a deposit with the same ID has been made to that client's account.
  2. Withdrawal
      - decreases the `available` amount
      - fails if available amount is less than the withdrawal amount.
  3. Dispute
      - only Deposits can be disputed
      - move disputed funds from `available` to `held`.
  4. Resolve
      - only disputed deposits can be resolved
      - move disputed funds from `held` to `available`.
  5. Chargeback
      - only disputed deposits can be charged back
      - `held`funds decrease by the disputed amount
      - account wallet gets **locked**.
### Running and input/output:
```
cargo run -- transactions.csv > accounts.csv
# or with warnings to `stderr`:
RUST_LOG=warn cargo run -- transactions.csv > accounts.csv
```
Input and output example:
```
transactions.csv
type, client, tx, amount
deposit, 1, 1, 5
dispute, 1, 1
chargeback, 1, 1
withdraw, 1, 2, 10

accounts.csv
client, available, held, total, locked
1, 0, 0, 0, true
```
