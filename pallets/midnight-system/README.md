# pallet-midnight-system

[FRAME](../../GLOSSARY.md#frame-framework-for-runtime-aggregation-of-modularized-entities) pallet for executing Midnight System Transactions with root privileges.

## Overview

This pallet provides a privileged interface for applying system-level transactions to the Midnight ledger. System transactions are generated from Cardano observations (e.g., [cNIGHT](../../GLOSSARY.md#cnight) registrations, [DUST](../../GLOSSARY.md#dust) generation) and require root origin to execute. The pallet implements `MidnightSystemTransactionExecutor` trait used by observation pallets.

## API Specification

### Dispatchables

| Call | Origin | Weight | Description |
|------|--------|--------|-------------|
| `send_mn_system_transaction` | Root | Configurable | Apply a serialized system transaction |

### Events

| Event | Fields | Description |
|-------|--------|-------------|
| `SystemTransactionApplied` | `hash: Hash`, `serialized_system_transaction: Vec<u8>` | System tx successfully applied |

### Errors

| Error | Description |
|-------|-------------|
| `LedgerApiError` | Wrapped ledger API error |

### Config Trait

| Associated Type | Constraint | Description |
|-----------------|------------|-------------|
| `LedgerStateProviderMut` | `LedgerStateProviderMut` | Access to ledger state |
| `LedgerBlockContextProvider` | `LedgerBlockContextProvider` | Block context (timestamp, hash) |

### Storage

| Name | Type | Description |
|------|------|-------------|
| `ConfigurableSystemTxWeight` | `Weight` | Processing weight for system transactions |

### Cargo Features

| Feature | Default | Description |
|---------|---------|-------------|
| `std` | Yes | Standard library support |
| `runtime-benchmarks` | No | Weight benchmarking |
| `try-runtime` | No | Migration testing |

## Usage

### Runtime Configuration

```rust
impl pallet_midnight_system::Config for Runtime {
    type LedgerStateProviderMut = Midnight;  // pallet-midnight
    type LedgerBlockContextProvider = Midnight;
}
```

### Via MidnightSystemTransactionExecutor Trait

From `pallets/midnight-system/src/lib.rs` (L94-L96):

```rust
use midnight_primitives::MidnightSystemTransactionExecutor;

// Signature: fn execute_system_transaction(Vec<u8>) -> Result<Hash, DispatchError>
let tx_hash = MidnightSystem::execute_system_transaction(serialized_tx)?;
```

## Architecture

```
Cardano Observation Flow:
+----------------------+     +--------------------+     +------------------+
| pallet-cnight-       | --> | MidnightSystem::   | --> | LedgerApi::      |
| observation          |     | execute_system_tx  |     | apply_system_tx  |
+----------------------+     +--------------------+     +------------------+
                                      |
                                      v
                             +--------------------+
                             | Event:             |
                             | SystemTxApplied    |
                             +--------------------+
```

## Integration

### Dependencies

- `midnight-node-ledger` - Ledger bridge API
- `midnight-primitives` - `MidnightSystemTransactionExecutor` trait

### Used By

- `pallet-cnight-observation` - Executes [DUST](../../GLOSSARY.md#dust) registration system transactions

## See Also

- [pallet-midnight](../midnight/README.md) - Core ledger pallet
- [pallet-cnight-observation](../cnight-observation/README.md) - Cardano bridge

