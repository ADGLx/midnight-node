# midnight-node-ledger

Bridge between Substrate runtime and Midnight's [ZSwap](../GLOSSARY.md#zswap) ledger.

## Overview

This crate provides host functions that allow the WASM runtime to interact with the Midnight ledger. It handles:

- **Transaction processing** - Apply and validate [ZSwap](../GLOSSARY.md#zswap) transactions
- **State management** - Read/write ledger state with caching
- **Versioning** - Support multiple ledger versions for hard forks

The crate uses module parameterization to support both current (`latest`) and hard-fork test (`hard_fork_test`) ledger versions simultaneously.

## Conditional Compilation

Compiling the ledger using the `HARDFORK_TEST` environment variable will change public exports to use the `hardfork_test` version of the ledger. The exports that are affected are:

```
midnight_node_ledger::types::active_version
midnight_node_ledger::types::active_ledger_bridge
```

When compiled for WASM (`no_std`), it only includes `host_api` and minimal stubs. When compiled for native (`std` feature), it includes full host function implementations + storage + json serialization.

## API Specification

### Host Functions (via `ledger_bridge`)

| Function | Description |
|----------|-------------|
| `apply_transaction` | Process a user transaction |
| `apply_system_transaction` | Process a system transaction (from observations) |
| `validate_transaction` | Validate without applying |
| `pre_fetch_storage` | Cache ledger state for block |
| `post_block_update` | Finalize block state |
| `flush_storage` | Persist state to disk |
| `get_contract_state` | Query contract state |
| `get_zswap_state_root` | Get ZSwap Merkle root |
| `mint_coins` | Mint block rewards |
| `get_version` | Get ledger library version |

### Types (via `types` module)

| Type | Description |
|------|-------------|
| `Tx` | Decoded transaction |
| `Hash` | 32-byte ledger hash |
| `BlockContext` | Timestamp and parent hash |
| `GasCost` / `StorageCost` | Transaction costs |
| `UtxoInfo` | UTXO details |
| `LedgerApiError` | Error variants |

### Version Modules

| Module | Cfg Flag | Description |
|--------|----------|-------------|
| `latest` | default | Current production ledger |
| `hard_fork_test` | `hardfork_test` | Test ledger for upgrades |
| `types::active_version` | - | Re-exports active version types |
| `types::active_ledger_bridge` | - | Re-exports active host functions |

### Cargo Features

| Feature | Default | Description |
|---------|---------|-------------|
| `std` | Yes | Native host function implementations |
| `runtime-benchmarks` | No | Benchmarking support |
| `test-utils` | Yes | Test utilities |

## Architecture

```
+------------------+     +------------------+     +------------------+
| pallet-midnight  | --> | ledger_bridge    | --> | mn-ledger        |
| (WASM runtime)   |     | (host functions) |     | (native library) |
+------------------+     +------------------+     +------------------+
                                 |
                                 v
                         +------------------+
                         | ledger-storage   |
                         | (ParityDB)       |
                         +------------------+
```

**Sources**: Host functions [`ledger/src/lib.rs`](https://github.com/m2ux/midnight-node/blob/mc_study/ledger/src/lib.rs), storage init [`node/src/service.rs#L217-L223`](https://github.com/m2ux/midnight-node/blob/mc_study/node/src/service.rs#L217-L223)

## Usage

### From Pallets

```rust
use midnight_node_ledger::types::{
    active_ledger_bridge as LedgerApi,
    active_version::LedgerApiError,
};

// Apply transaction
let result = LedgerApi::apply_transaction(
    &state_key,
    &tx_bytes,
    block_context,
    runtime_version,
)?;

// Get contract state
let state = LedgerApi::get_contract_state(&state_key, &contract_addr)?;
```

### JSON Transformation (std only)

The json module provides utilities for transforming JSON values (from `ledger/src/json.rs`):

```rust
use midnight_node_ledger::json;
use serde_json::Value;

// Transform byte arrays in JSON to hex strings
let json_value: Value = serde_json::from_str(&json_str)?;
let transformed = json::transform(json_value);
```

The `transform` function converts byte arrays to hex strings (e.g., `[0x24, 0x42]` → `"0x2442"`).

## Integration

### Dependencies (Native)

- `mn-ledger` / `mn-ledger-hf` - Core ledger logic
- `ledger-storage` - RocksDB state storage
- `zswap` - ZSwap proving system
- `midnight-node-ledger-helpers` - Test utilities

### Used By

- `pallet-midnight` - Transaction processing
- `pallet-midnight-system` - System transactions
- `midnight-node` - Storage initialization

## Testing

```bash
cargo test -p midnight-node-ledger
```

## See Also

- [ledger/helpers](helpers/README.md) - Test utilities
- [pallet-midnight](../pallets/midnight/README.md) - Primary consumer
- [primitives/ledger](../primitives/ledger/README.md) - Metrics primitives
