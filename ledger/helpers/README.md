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

- [**`latest`**](https://github.com/midnightntwrk/midnight-node/blob/main/ledger/helpers/src/latest/) - Current ledger version helpers (default)
- [**`hard_fork_test`**](https://github.com/midnightntwrk/midnight-node/blob/main/ledger/helpers/src/hard_fork_test/) - Hard fork test helpers

### Re-exported Crates (std only)

Each version module re-exports:

- **`mn_ledger`** - Core ledger types and logic
- **`base_crypto`** - Cryptographic primitives
- **`coin_structure`** - UTXO and coin types
- **`zswap`** - ZSwap proving system
- **`zkir`** - Zero-knowledge IR
- **`ledger_storage`** - Storage interface
- **`onchain_runtime`** - On-chain execution

### Common Utilities

- [**`find_dependency_version`**](https://github.com/midnightntwrk/midnight-node/blob/main/ledger/helpers/src/lib.rs#L30) - Get version string for a dependency

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

- `pallet-midnight` tests
- `pallet-cnight-observation` tests
- `tests/e2e` - End-to-end tests
- `util/toolkit` - CLI tools

## Testing

```bash
cargo test -p midnight-node-ledger-helpers
```

## See Also

- [ledger](../README.md) - Parent ledger crate
- [tests/e2e](../../tests/e2e/README.md) - Integration tests

