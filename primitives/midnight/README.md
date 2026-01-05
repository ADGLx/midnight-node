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

| Trait | Description |
|-------|-------------|
| `LedgerStateProviderMut` | Provides mutable access to ledger state |
| `LedgerBlockContextProvider` | Provides block context (timestamp, parent hash) |
| `MidnightSystemTransactionExecutor` | Executes system transactions from observations |

### LedgerStateProviderMut

```rust
pub trait LedgerStateProviderMut {
    /// Get the current ledger state key (root hash)
    fn get_ledger_state_key() -> Vec<u8>;
    
    /// Mutate ledger state with a closure, returning updated key and result
    fn mut_ledger_state<F, E, R>(f: F) -> Result<R, E>
    where
        F: FnOnce(Vec<u8>) -> Result<(Vec<u8>, R), E>;
}
```

### LedgerBlockContextProvider

```rust
pub trait LedgerBlockContextProvider {
    fn get_block_context() -> BlockContext;
}
```

### MidnightSystemTransactionExecutor

```rust
pub trait MidnightSystemTransactionExecutor {
    fn execute_system_transaction(
        serialized_system_transaction: Vec<u8>,
    ) -> Result<Hash, DispatchError>;
}
```

### Transaction Types

```rust
pub enum TransactionTypeV2 {
    MidnightTx(Vec<u8>, Result<Tx, LedgerApiError>),
    TimestampTx(u64),
    UnknownTx,
}
```

### Well-Known Keys

| Key | Purpose |
|-----|---------|
| `MIDNIGHT_STATE_KEY` | Storage key for ledger state root |
| `MIDNIGHT_NETWORK_ID_KEY` | Storage key for network identifier |

### Cargo Features

| Feature | Default | Description |
|---------|---------|-------------|
| `std` | Yes | Standard library support |

## Usage

### Implementing LedgerStateProviderMut

```rust
impl LedgerStateProviderMut for MyPallet {
    fn get_ledger_state_key() -> Vec<u8> {
        StateKey::get().expect("state initialized").into()
    }
    
    fn mut_ledger_state<F, E, R>(f: F) -> Result<R, E>
    where F: FnOnce(Vec<u8>) -> Result<(Vec<u8>, R), E>
    {
        let state_key = StateKey::get().expect("state initialized");
        let (new_key, result) = f(state_key.into())?;
        StateKey::put(new_key.try_into().expect("valid"));
        Ok(result)
    }
}
```

### Using Well-Known Keys

From `primitives/midnight/src/lib.rs` (L59-L67):

```rust
use midnight_primitives::well_known_keys::{MIDNIGHT_STATE_KEY, MIDNIGHT_NETWORK_ID_KEY};

// Key values (32-byte hashes):
// MIDNIGHT_STATE_KEY:      0x2a760f9a...021f43d9c
// MIDNIGHT_NETWORK_ID_KEY: 0x2a760f9a...efce9fc4

// Direct storage access (e.g., in host functions)
let state = sp_io::storage::get(MIDNIGHT_STATE_KEY);
let network_id = sp_io::storage::get(MIDNIGHT_NETWORK_ID_KEY);
```

## Integration

### Dependencies

- `midnight-node-ledger` - Ledger types (`BlockContext`, `Hash`, `Tx`)
- `sp-runtime` - `DispatchError`

### Used By

- `pallet-midnight` - Implements all traits
- `pallet-midnight-system` - Uses `LedgerStateProviderMut`
- `pallet-cnight-observation` - Uses `MidnightSystemTransactionExecutor`

## See Also

- [pallet-midnight](../pallets/midnight/README.md) - Primary implementor
- [ledger](../ledger/README.md) - Ledger types

