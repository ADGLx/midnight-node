# Toolkit Architecture

## Overview

The toolkit is a transaction generator and testing tool for Midnight. It fetches chain state, builds transactions against a local ledger context, and submits them to a node.

```
                              User
                               │
                               ▼
                    ┌──────────────────────┐
                    │   CLI                │
                    │   cli.rs, commands/  │
                    └──────────────────────┘
                               │
            ┌──────────────────┼──────────────────┐
            ▼                  ▼                   ▼
     TxGenerator           toolkit_js           Genesis
  (tx_generator/mod.rs)  (toolkit_js/mod.rs)  (genesis_generator.rs)
            │                  │
            │                  ▼
            │         Node.js child process
            │         toolkit.js -> compact.js → compact-runtime
            │         Deploy | Circuit | Maintain
            │
            ▼
 ┌───────────────────────────────────────────┐
 │  Source (source.rs)                       │
 │  GetTxsFromFile (.mn)  GetTxsFromUrl      │
 └───────────────────────────────────────────┘
            │                          │
       from file                  from node
            │                          │
            │                          ▼
            │          ┌──────────────────────────────────┐
            │          │  Fetcher (fetcher.rs, fetcher/)  │
            │          │  Parallel block fetch + extract  │
            │          │  Runtime dispatch (runtimes.rs)  │
            │          │  Storage: InMemory|ReDB|Postgres │
            │          └──────────────────────────────────┘
            │                          │
            └──────────┬───────────────┘
                       │
                       ▼  SourceTransactions
 ┌───────────────────────────────────────────┐
 │  Builder (tx_generator/builder/)          │
 │  ForkAwareLedgerContext (Ledger7|Ledger8) │
 │  Dual-compiled common/ builders           │
 │  Prover: Local | Remote (remote_prover.rs)│
 └───────────────────────────────────────────┘
                       │
                       ▼  SerializedTxBatches
 ┌───────────────────────────────────────────┐
 │  Destination (destination.rs)             │
 │  SendTxsToFile (.mn)   SendTxsToUrl       │
 └───────────────────────────────────────────┘
                       │
                       ▼
 ┌───────────────────────────────────────────┐
 │  Midnight Node (RPC :9944)               │
 │  MidnightNodeClient (client.rs)          │
 │  Rate-limited by Sender (sender.rs)      │
 └───────────────────────────────────────────┘
```

## Components

| Component | Re-implements | Description |
|-----------|---------------|-------------|
| Fetcher | Indexer | Parallel block fetching, extraction, and caching with pluggable storage backends |
| Builder | Wallet | Ledger context management, wallet state, proof generation |
| toolkit_js | midnight-js | Thin wrapper around compact.js (which wraps the compact-runtime) via Node.js bridge; Deploy/Circuit/Maintain |
| CLI | -- | Command dispatch with fork-aware subcommands (`commands/fork/ledger_7.rs`, `ledger_8.rs`) |
| TxGenerator | -- | Source → Builder → Destination pipeline |

## Fetcher Pipeline

The fetcher (`fetcher.rs`) orchestrates parallel block fetching and extraction via `fetch_all()`:

1. **Job Pusher** -- iterates block heights in steps of `BLOCKS_PER_JOB`, pushes fetch tasks onto a bounded channel
2. **Fetch Workers** (N) -- each with its own RPC client; checks storage cache, fetches missing blocks with exponential backoff (`fetch_task.rs`)
3. **Compute Workers** (M) -- extract block data via runtime version dispatch (`runtimes.rs`), verify hash chains, store results (`compute_task.rs`)
4. **Collector** -- gathers verified results, returns ordered blocks

Storage backends (`fetcher/fetch_storage/`): InMemory, ReDB, PostgreSQL.

Wallet state is cached across runs via `wallet_state_cache.rs`.

## Dual Compilation

The `ledger-helpers` crate (`util/ledger-helpers/`) defines the transaction building primitives (wallet, ledger context, proof providers) used by the builders. It is itself dual-compiled -- `ledger_7` and `ledger_8` modules expose the same interface against different ledger versions.

Both `builders/ledger_7.rs` and `builders/ledger_8.rs` use the same `common/` source files, compiled twice with different type aliases:

```rust
#[path = "common"]
pub mod inner {
    pub use midnight_node_ledger_helpers::ledger_N as ledger_helpers_local;
    // ... mod declarations for each builder
}
```

The `common/*.rs` files reference `ledger_helpers_local` for all ledger types. Because the alias resolves to the version-specific module before compilation, each file compiles against the correct ledger version's types.

The same pattern is used in `commands/fork/` for fork-aware read-only commands.

### Ledger 7 Limitations

- **No Remote Prover** -- returns `BuilderConstructionError::RemoteProverNotSupportedForLedger7`
- **Auto-transition** -- `update_from_block()` detects when a Ledger7 context receives a Ledger8 block and calls `next_fork()` → `fork_context_7_to_8()` to migrate state
