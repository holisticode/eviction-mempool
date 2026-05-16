# eviction-mempool: Priority-Fee Mempool

A priority-fee based transaction mempool implemented in Rust, built as a learning project for protocol engineering. Demonstrates core data structure design, concurrency patterns, and protocol correctness properties found in production implementations like Reth.

A training exercise to improve rust skills.

## Architecture

Transactions are stored in a BTreeMap<u64, Vec<Transaction>> keyed by fee. This gives O(log n) access to both the highest-fee transactions (block building) and the lowest-fee transactions (eviction), which a single BinaryHeap cannot provide efficiently.

The pool is shared across concurrent tasks via Arc<RwLock<Mempool>> — a block builder task snapshots the top N transactions every 500ms while a separate task continuously ingests new transactions.

## Features

Priority ordering — transactions sorted by fee, highest first
Eviction — when at capacity, incoming transactions that outbid the lowest-fee entry evict it
Duplicate detection — replace-by-fee: a transaction can replace a pending one with the same sender + nonce if it pays a higher fee
Nonce gap detection — enforces per-sender nonce ordering against both pool state and on-chain state
State validation — pluggable StateProvider trait for validating sender balance and on-chain nonce before insertion; chain-agnostic by design
Eviction notifications — broadcast channel for fire-and-forget eviction events
Metrics — atomic counters for eviction count, rejection count, and average fee in pool
Concurrent block builder — mock builder task that periodically snapshots the pool without blocking ingestion

## Project Structure

```
src/
├── main.rs          # tokio runtime, spawns builder and injector tasks
├── mempool.rs       # BTreeMap-backed pool: insert, pop_best, pop_worst, snapshot
├── mempool_tests.rs # unit tests for all operations and failure modes
├── transaction.rs   # Transaction struct
├── builder.rs       # mock block builder task
├── state.rs         # StateProvider trait + MockStateProvider
└── errors.rs        # MempoolError enum with structured variants
```

## Running

```bash
cargo run    # starts concurrent builder + injector tasks
```

```bash
cargo test   # runs 19 unit tests
```
