# Primitives

Shared types and traits used across runtime and native code.

## Overview

```
+-----------------------------------------------------------------------+
|                            Primitives                                  |
+-----------------------------------------------------------------------+
| midnight/              | Core traits and transaction types            |
| ledger/                | Metrics and externality extensions           |
| cnight-observation/    | Cardano UTXO observation types               |
| federated-authority-*/ | Governance observation types                 |
| mainchain-follower/    | Data source interface types                  |
+-----------------------------------------------------------------------+
```

## Packages

### [midnight/](midnight/README.md)
**midnight-primitives** - Core traits (`LedgerStateProviderMut`, `LedgerBlockContextProvider`, `MidnightSystemTransactionExecutor`) and transaction types.

### [ledger/](ledger/README.md)
**midnight-primitives-ledger** - Prometheus metrics for ledger operations and externality extensions for host functions.

### [cnight-observation/](cnight-observation/README.md)
**midnight-primitives-cnight-observation** - Types for Cardano UTXO observation: `CardanoPosition`, `ObservedUtxo`, registration/redemption data structures.

### [federated-authority-observation/](federated-authority-observation/README.md)
**midnight-primitives-federated-authority-observation** - Types for governance body membership observation from Cardano.

### [mainchain-follower/](mainchain-follower/README.md)
**midnight-primitives-mainchain-follower** - Data source interface types and re-exports for mainchain observation.

## Package Index

| Package | Path | Description |
|---------|------|-------------|
| `midnight-primitives` | `midnight/` | Core primitives |
| `midnight-primitives-ledger` | `ledger/` | Ledger primitives |
| `midnight-primitives-cnight-observation` | `cnight-observation/` | cNIGHT types |
| `midnight-primitives-federated-authority-observation` | `federated-authority-observation/` | Governance types |
| `midnight-primitives-mainchain-follower` | `mainchain-follower/` | Mainchain types |

## See Also

- [pallets/](../pallets/README.md) - Pallets that use these primitives
- [runtime/](../runtime/README.md) - Runtime integration
- [Glossary](https://docs.midnight.network/learn/glossary) - Domain-specific terminology

