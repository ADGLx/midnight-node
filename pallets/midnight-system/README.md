# pallet-midnight-system

[FRAME](https://docs.midnight.network/learn/glossary#frame-framework-for-runtime-aggregation-of-modularized-entities) pallet for executing Midnight System Transactions with root privileges.

## Overview

This pallet provides a privileged interface for applying system-level transactions to the Midnight ledger. System transactions are generated from Cardano observations (e.g., [cNIGHT](https://docs.midnight.network/learn/glossary#cnight) registrations, [DUST](https://docs.midnight.network/learn/glossary#dust) generation) and require root origin to execute. The pallet implements `MidnightSystemTransactionExecutor` trait used by observation pallets.

## API Specification

### Dispatchables

- [**`send_mn_system_transaction`**](https://github.com/midnightntwrk/midnight-node/blob/main/pallets/midnight-system/src/lib.rs#L93) - Apply a serialized system transaction

### Events

- [**`SystemTransactionApplied`**](https://github.com/midnightntwrk/midnight-node/blob/main/pallets/midnight-system/src/lib.rs#L70) - System tx successfully applied

### Errors

- [**`LedgerApiError`**](https://github.com/midnightntwrk/midnight-node/blob/main/pallets/midnight-system/src/lib.rs#L78) - Wrapped ledger API error

### Config Trait

- [**`LedgerStateProviderMut`**](https://github.com/midnightntwrk/midnight-node/blob/main/pallets/midnight-system/src/lib.rs#L55) - Access to ledger state
- [**`LedgerBlockContextProvider`**](https://github.com/midnightntwrk/midnight-node/blob/main/pallets/midnight-system/src/lib.rs#L58) - Block context (timestamp, hash)

### Storage

- [**`ConfigurableSystemTxWeight`**](https://github.com/midnightntwrk/midnight-node/blob/main/pallets/midnight-system/src/lib.rs#L65) - Processing weight for system transactions

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

**Sources**: [[1]](https://github.com/midnightntwrk/midnight-node/blob/main/pallets/midnight-system/src/lib.rs#L93-L120)

## Integration

### Dependencies

- `midnight-node-ledger` - Ledger bridge API
- `midnight-primitives` - `MidnightSystemTransactionExecutor` trait

### Used By

- `pallet-cnight-observation` - Executes [DUST](https://docs.midnight.network/learn/glossary#dust) registration system transactions

## See Also

- [pallet-midnight](../midnight/README.md) - Core ledger pallet
- [pallet-cnight-observation](../cnight-observation/README.md) - Cardano bridge

