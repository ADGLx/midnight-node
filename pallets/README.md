# Pallets

Custom FRAME pallets implementing Midnight-specific blockchain logic.

## Overview

```
+-----------------------------------------------------------------------+
|                              Pallets                                   |
+-----------------------------------------------------------------------+
| Core                           | Governance                            |
|   midnight (ledger/tx)         |   federated-authority                 |
|   midnight-system (sys tx)     |   federated-authority-observation     |
|   cnight-observation (bridge)  |                                       |
|   version (tracking)           |                                       |
+-----------------------------------------------------------------------+
```

## Core Pallets

### [midnight/](midnight/README.md)
**pallet-midnight** - Core ledger pallet managing ZSwap state, transaction processing, contract deployment/calls, and block rewards.

### [midnight/rpc/](midnight/rpc/README.md)
**pallet-midnight-rpc** - JSON-RPC interface exposing `midnight_contractState`, `midnight_zswapStateRoot`, and `midnight_ledgerVersion` methods.

### [midnight-system/](midnight-system/README.md)
**pallet-midnight-system** - Privileged system transaction executor for applying Cardano-observed state changes (registrations, DUST generation).

### [cnight-observation/](cnight-observation/README.md)
**pallet-cnight-observation** - Cardano bridge pallet observing cNIGHT token movements, wallet registrations, and Glacier Drop redemptions.

### [cnight-observation/mock/](cnight-observation/mock/README.md)
**pallet-cnight-observation-mock** - Test mock runtime for cNIGHT observation pallet unit testing.

### [version/](version/README.md)
**pallet-version** - Records runtime spec version in block digests for external monitoring and upgrade tracking.

## Governance Pallets

### [federated-authority/](federated-authority/README.md)
**pallet-federated-authority** - Cross-collective governance mechanism requiring multi-body approval before executing privileged operations.

### [federated-authority-observation/](federated-authority-observation/README.md)
**pallet-federated-authority-observation** - Observes and propagates Council/Technical Committee membership changes from Cardano.

## Package Index

| Package | Path | Description |
|---------|------|-------------|
| `pallet-midnight` | `midnight/` | Core ledger pallet |
| `pallet-midnight-rpc` | `midnight/rpc/` | RPC interface |
| `pallet-midnight-system` | `midnight-system/` | System transactions |
| `pallet-cnight-observation` | `cnight-observation/` | Cardano bridge |
| `pallet-cnight-observation-mock` | `cnight-observation/mock/` | Test mock |
| `pallet-version` | `version/` | Version logging |
| `pallet-federated-authority` | `federated-authority/` | Multi-body governance |
| `pallet-federated-authority-observation` | `federated-authority-observation/` | Governance observation |

## See Also

- [runtime/](../runtime/README.md) - Runtime that wires these pallets together
- [primitives/](../primitives/README.md) - Shared types used by pallets
- [Glossary](https://docs.midnight.network/learn/glossary) - Domain-specific terminology

