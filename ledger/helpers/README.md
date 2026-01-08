# midnight-node-ledger-helpers

Test utilities and helpers for working with the Midnight ledger.

## Overview

This crate provides high-level utilities for:

- **Transaction building** - Create test transactions with proofs
- **Wallet operations** - Key derivation and address generation
- **State inspection** - Query and verify ledger state
- **Version support** - Helpers for both `latest` and `hard_fork_test` versions

## API Specification

### Version Modules

- [**`latest`**](https://github.com/midnightntwrk/midnight-node/blob/main/ledger/helpers/src/lib.rs#L33) - Current ledger version helpers
- [**`hard_fork_test`**](https://github.com/midnightntwrk/midnight-node/blob/main/ledger/helpers/src/lib.rs#L19) - Hard fork test helpers (with `hardfork_test` cfg flag)

### Re-exported Crates (std only)

Each version module re-exports:

- **`mn_ledger`** - Core ledger types and logic (external)
- **`base_crypto`** - Cryptographic primitives (external)
- **`coin_structure`** - UTXO and coin types (external)
- **`zswap`** - ZSwap proving system (external)
- **`zkir`** - Zero-knowledge IR (external)
- **`ledger_storage`** - Storage interface (external)
- **`onchain_runtime`** - On-chain execution (external)

### Common Utilities

- [**`find_dependency_version`**](https://github.com/midnightntwrk/midnight-node/blob/main/ledger/helpers/src/utils.rs#L45) - Get version string for a dependency

## Usage

### Building Test Transactions

```rust
use midnight_node_ledger_helpers::latest::*;

// Create a wallet from mnemonic
let wallet = Wallet::from_mnemonic("...", network)?;

// Build a contract call
let tx = TxBuilder::new()
    .call(contract_addr, "entrypoint", args)
    .sign(&wallet)
    .build()?;
```

### Version Inspection

```rust
use midnight_node_ledger_helpers::find_dependency_version;

let version = find_dependency_version("mn-ledger")?;
println!("Using ledger version: {}", version);
```

## Integration

### Dependencies

- `mn-ledger` / `mn-ledger-hf` - Ledger with test utilities
- `bip32` / `bip39` - Key derivation
- `rand` - Random number generation

### Used By

- [`pallet-midnight`](https://github.com/midnightntwrk/midnight-node/blob/main/pallets/midnight/src/lib.rs) tests
- [`pallet-cnight-observation`](https://github.com/midnightntwrk/midnight-node/blob/main/pallets/cnight-observation/src/lib.rs) tests
- [`tests/e2e`](https://github.com/midnightntwrk/midnight-node/blob/main/tests/e2e/src/lib.rs) - End-to-end tests
- [`util/toolkit`](https://github.com/midnightntwrk/midnight-node/blob/main/util/toolkit/src/lib.rs) - CLI tools

## Testing

```bash
cargo test -p midnight-node-ledger-helpers
```

## See Also

- [ledger](../README.md) - Parent ledger crate
- [tests/e2e](../../tests/e2e/README.md) - Integration tests

