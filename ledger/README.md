# midnight-node-ledger

Bridge between Substrate runtime and Midnight's [ZSwap](https://docs.midnight.network/learn/glossary#zswap) ledger.

## Overview

This crate provides host functions that allow the WASM runtime to interact with the Midnight ledger. It handles:

- **Transaction processing** - Apply and validate [ZSwap](https://docs.midnight.network/learn/glossary#zswap) transactions
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

- [**`apply_transaction`**](https://github.com/midnightntwrk/midnight-node/blob/main/ledger/src/lib.rs#L100) - Process a user transaction
- [**`apply_system_transaction`**](https://github.com/midnightntwrk/midnight-node/blob/main/ledger/src/lib.rs#L120) - Process a system transaction (from observations)
- [**`validate_transaction`**](https://github.com/midnightntwrk/midnight-node/blob/main/ledger/src/lib.rs#L140) - Validate without applying
- [**`pre_fetch_storage`**](https://github.com/midnightntwrk/midnight-node/blob/main/ledger/src/lib.rs#L160) - Cache ledger state for block
- [**`post_block_update`**](https://github.com/midnightntwrk/midnight-node/blob/main/ledger/src/lib.rs#L180) - Finalize block state
- [**`flush_storage`**](https://github.com/midnightntwrk/midnight-node/blob/main/ledger/src/lib.rs#L200) - Persist state to disk
- [**`get_contract_state`**](https://github.com/midnightntwrk/midnight-node/blob/main/ledger/src/lib.rs#L220) - Query contract state
- [**`get_zswap_state_root`**](https://github.com/midnightntwrk/midnight-node/blob/main/ledger/src/lib.rs#L240) - Get ZSwap Merkle root
- [**`mint_coins`**](https://github.com/midnightntwrk/midnight-node/blob/main/ledger/src/lib.rs#L260) - Mint block rewards
- [**`get_version`**](https://github.com/midnightntwrk/midnight-node/blob/main/ledger/src/lib.rs#L280) - Get ledger library version

### Types (via `types` module)

- [**`Tx`**](https://github.com/midnightntwrk/midnight-node/blob/main/ledger/src/types.rs#L20) - Decoded transaction
- [**`Hash`**](https://github.com/midnightntwrk/midnight-node/blob/main/ledger/src/types.rs#L30) - 32-byte ledger hash
- [**`BlockContext`**](https://github.com/midnightntwrk/midnight-node/blob/main/ledger/src/types.rs#L40) - Timestamp and parent hash
- [**`GasCost`**](https://github.com/midnightntwrk/midnight-node/blob/main/ledger/src/types.rs#L50) / [**`StorageCost`**](https://github.com/midnightntwrk/midnight-node/blob/main/ledger/src/types.rs#L55) - Transaction costs
- [**`UtxoInfo`**](https://github.com/midnightntwrk/midnight-node/blob/main/ledger/src/types.rs#L60) - UTXO details
- [**`LedgerApiError`**](https://github.com/midnightntwrk/midnight-node/blob/main/ledger/src/types.rs#L70) - Error variants

### Version Modules

- [**`latest`**](https://github.com/midnightntwrk/midnight-node/blob/main/ledger/src/latest/) - Current production ledger (default)
- [**`hard_fork_test`**](https://github.com/midnightntwrk/midnight-node/blob/main/ledger/src/hard_fork_test/) - Test ledger for upgrades
- **`types::active_version`** - Re-exports active version types
- **`types::active_ledger_bridge`** - Re-exports active host functions

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

**Sources**: [[1]](https://github.com/midnightntwrk/midnight-node/blob/main/ledger/src/lib.rs) [[2]](https://github.com/midnightntwrk/midnight-node/blob/main/node/src/service.rs#L217-L223)

## JSON Transformation

The `json::transform` function converts byte arrays to hex strings (e.g., `[0x24, 0x42]` → `"0x2442"`).

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
