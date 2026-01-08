# midnight-primitives

Core primitive types and traits shared across Midnight node components.

## Overview

This crate defines the fundamental abstractions for Midnight's ledger interaction:

- **Traits** for ledger state mutation and [block context](https://docs.midnight.network/learn/glossary#block-context) access
- **Transaction type enums** for runtime transaction classification
- **Well-known storage keys** for direct state access

These primitives are `no_std` compatible for use in both native and WASM runtime contexts.

## API Specification

### Traits

- [**`LedgerStateProviderMut`**](https://github.com/midnightntwrk/midnight-node/blob/main/primitives/midnight/src/lib.rs#L26) - Provides mutable access to ledger state
- [**`LedgerBlockContextProvider`**](https://github.com/midnightntwrk/midnight-node/blob/main/primitives/midnight/src/lib.rs#L35) - Provides block context (timestamp, parent hash)
- [**`MidnightSystemTransactionExecutor`**](https://github.com/midnightntwrk/midnight-node/blob/main/primitives/midnight/src/lib.rs#L39) - Executes system transactions from observations

### Well-Known Keys

- [**`MIDNIGHT_STATE_KEY`**](https://github.com/midnightntwrk/midnight-node/blob/main/primitives/midnight/src/lib.rs#L167) - Storage key for ledger state root
- [**`MIDNIGHT_NETWORK_ID_KEY`**](https://github.com/midnightntwrk/midnight-node/blob/main/primitives/midnight/src/lib.rs#L170) - Storage key for network identifier

## Integration

### Dependencies

- `midnight-node-ledger` - Ledger types (`BlockContext`, `Hash`, `Tx`)
- `sp-runtime` - `DispatchError`

### Used By

- [`pallet-midnight`](https://github.com/midnightntwrk/midnight-node/blob/main/pallets/midnight/src/lib.rs) - Implements all traits
- [`pallet-midnight-system`](https://github.com/midnightntwrk/midnight-node/blob/main/pallets/midnight-system/src/lib.rs) - Uses `LedgerStateProviderMut`
- [`pallet-cnight-observation`](https://github.com/midnightntwrk/midnight-node/blob/main/pallets/cnight-observation/src/lib.rs) - Uses `MidnightSystemTransactionExecutor`

## See Also

- [pallet-midnight](../pallets/midnight/README.md) - Primary implementor
- [ledger](../ledger/README.md) - Ledger types

