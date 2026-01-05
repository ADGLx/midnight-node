# Primitives

Shared types and traits used across runtime and native code.

## Overview

This directory contains foundational types, traits, and data structures that are shared between the [WASM](https://docs.polkadot.com/polkadot-protocol/glossary/#webassembly-wasm) runtime and native node code. Primitives define the interfaces between pallets and the native ledger implementation, enabling type-safe communication across the WASM boundary.

The primitives are organized by domain: core traits for ledger state access, observation types for Cardano bridge data, and metrics/extensions for operational monitoring. These crates are dependencies of both the runtime (compiled to WASM) and the node (compiled natively), so they must be `no_std` compatible.

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

## See Also

- [pallets/](../pallets/README.md) - Pallets that use these primitives
- [runtime/](../runtime/README.md) - Runtime integration
- [Glossary](https://docs.midnight.network/learn/glossary) - Domain-specific terminology

