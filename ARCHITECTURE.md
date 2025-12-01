# Midnight Node Architecture

This document provides an index of all packages and components in the Midnight Node repository, organized by functional area.

## Overview

```
+------------------------------------------------------------------+
|                         Midnight Node                            |
+------------------------------------------------------------------+
|                                                                  |
|  +------------+  +------------+  +------------+  +------------+  |
|  |   Node     |  |  Runtime   |  |  Pallets   |  | Primitives |  |
|  +------------+  +------------+  +------------+  +------------+  |
|       |              |              |              |              |
|       v              v              v              v              |
|  +------------+  +------------+  +------------+  +------------+  |
|  |  Ledger    |  |    Res     |  |  Metadata  |  |   Utils    |  |
|  +------------+  +------------+  +------------+  +------------+  |
|                                                                  |
+------------------------------------------------------------------+
```

---

## Core Components

### [README.md](README.md)
**Midnight Node** - Main project documentation with architecture diagrams, build instructions, and development guidelines.

### [node/](node/README.md)
**midnight-node** - Main node executable integrating consensus (AURA/GRANDPA/BEEFY), runtime execution, RPC server, and Cardano data source connections.

### [runtime/](runtime/README.md)
**midnight-node-runtime** - Substrate FRAME runtime defining all pallets, storage, and runtime APIs. The core blockchain logic compiled to WASM.

### [runtime/common/](runtime/common/README.md)
**runtime-common** - Shared governance utilities including `MembershipHandler`, `AlwaysNo` voting default, and `MembershipObservationHandler`.

---

## Pallets

Custom FRAME pallets implementing Midnight-specific blockchain logic.

### [pallets/midnight/](pallets/midnight/README.md)
**pallet-midnight** - Core ledger pallet managing ZSwap state, transaction processing, contract deployment/calls, and block rewards.

### [pallets/midnight/rpc/](pallets/midnight/rpc/README.md)
**pallet-midnight-rpc** - JSON-RPC interface exposing `midnight_contractState`, `midnight_zswapStateRoot`, and `midnight_ledgerVersion` methods.

### [pallets/midnight-system/](pallets/midnight-system/README.md)
**pallet-midnight-system** - Privileged system transaction executor for applying Cardano-observed state changes (registrations, DUST generation).

### [pallets/cnight-observation/](pallets/cnight-observation/README.md)
**pallet-cnight-observation** - Cardano bridge pallet observing cNIGHT token movements, wallet registrations, and Glacier Drop redemptions.

### [pallets/cnight-observation/mock/](pallets/cnight-observation/mock/README.md)
**pallet-cnight-observation-mock** - Test mock runtime for cNIGHT observation pallet unit testing.

### [pallets/version/](pallets/version/README.md)
**pallet-version** - Records runtime spec version in block digests for external monitoring and upgrade tracking.

### [pallets/federated-authority/](pallets/federated-authority/README.md)
**pallet-federated-authority** - Cross-collective governance mechanism requiring multi-body approval before executing privileged operations.

### [pallets/federated-authority-observation/](pallets/federated-authority-observation/README.md)
**pallet-federated-authority-observation** - Observes and propagates Council/Technical Committee membership changes from Cardano.

---

## Primitives

Shared types and traits used across runtime and native code.

### [primitives/midnight/](primitives/midnight/README.md)
**midnight-primitives** - Core traits (`LedgerStateProviderMut`, `LedgerBlockContextProvider`, `MidnightSystemTransactionExecutor`) and transaction types.

### [primitives/ledger/](primitives/ledger/README.md)
**midnight-primitives-ledger** - Prometheus metrics for ledger operations and externality extensions for host functions.

### [primitives/cnight-observation/](primitives/cnight-observation/README.md)
**midnight-primitives-cnight-observation** - Types for Cardano UTXO observation: `CardanoPosition`, `ObservedUtxo`, registration/redemption data structures.

### [primitives/federated-authority-observation/](primitives/federated-authority-observation/README.md)
**midnight-primitives-federated-authority-observation** - Types for governance body membership observation from Cardano.

### [primitives/mainchain-follower/](primitives/mainchain-follower/README.md)
**midnight-primitives-mainchain-follower** - Data source interface types and re-exports for mainchain observation.

---

## Ledger Integration

Bridge between Substrate runtime and Midnight's ZSwap ledger.

### [ledger/](ledger/README.md)
**midnight-node-ledger** - Host function implementations for transaction processing, state management, and ledger versioning (latest/hard-fork).

### [ledger/helpers/](ledger/helpers/README.md)
**midnight-node-ledger-helpers** - Test utilities for transaction building, wallet operations, and state verification.

---

## Resources & Configuration

### [res/](res/README.md)
**midnight-node-res** - Chain configuration resources: genesis data, network definitions, serialization utilities, and test fixtures.

### [metadata/](metadata/README.md)
**midnight-node-metadata** - Generated subxt interfaces and SCALE-encoded metadata for supported protocol versions.

---

## Utilities

### [util/toolkit/](util/toolkit/README.md)
**midnight-toolkit** - Feature-complete CLI for wallet management, transaction generation, contract deployment, and blockchain interaction.

### [util/toolkit-js/](util/toolkit-js/README.md)
**toolkit-js** - JavaScript/TypeScript CLI for executing compiled Compact contracts with witness implementations.

### [util/upgrader/](util/upgrader/README.md)
**upgrader** - HTTP service for triggering runtime upgrades via REST API, used in CI/CD and testing.

### [util/documented/](util/documented/README.md)
**documented** - Procedural macro workspace for extracting doc comments at runtime.

### [util/documented/documented_types/](util/documented/documented_types/README.md)
**documented_types** - Core `Documented` trait definitions.

### [util/documented/documented_proc_macro/](util/documented/documented_proc_macro/README.md)
**documented_proc_macro** - `#[derive(Documented)]` macro implementation.

---

## Relay & Bridge

### [relay/](relay/README.md)
**midnight-beefy-relay** - BEEFY key management and relay service for cross-chain bridge operations.

---

## Testing

### [tests/e2e/](tests/e2e/README.md)
**e2e** - End-to-end integration tests exercising the full node stack. Run separately from unit tests.

### [tests/redemption-skeleton/](tests/redemption-skeleton/README.md)
Test fixture for Glacier Drop redemption contract validation.

---

## Documentation

### [docs/](docs/README.md)
**docs** - Documentation crate for compile-time doc testing plus markdown documentation files.

### [docs/signatures/](docs/signatures/README.md)
GPG signatures for release verification.

---

## Development Environment

### [local-environment/](local-environment/README.md)
**Midnight Network Tools** - Docker-based tooling for launching local networks (qanet, devnet, testnet) and performing state operations.

### [ui/](ui/README.md)
**Polkadot SDK Front End Template** - Barebones React UI for interacting with the Midnight blockchain.

### [scripts/cnight-generates-dust/](scripts/cnight-generates-dust/README.md)
Test scripts for cNIGHT → DUST generation scenarios.

### [res/test-zswap/](res/test-zswap/README.md)
ZSwap test fixtures and scenarios.

---

## Package Index by Type

### Cargo Workspace Members

| Package | Path | Description |
|---------|------|-------------|
| `midnight-node` | `node/` | Main node executable |
| `midnight-node-runtime` | `runtime/` | WASM runtime |
| `runtime-common` | `runtime/common/` | Shared governance utilities |
| `midnight-node-res` | `res/` | Chain resources |
| `midnight-node-ledger` | `ledger/` | Ledger bridge |
| `midnight-node-ledger-helpers` | `ledger/helpers/` | Ledger test utilities |
| `midnight-node-metadata` | `metadata/` | Subxt metadata |
| `pallet-midnight` | `pallets/midnight/` | Core ledger pallet |
| `pallet-midnight-rpc` | `pallets/midnight/rpc/` | RPC interface |
| `pallet-midnight-system` | `pallets/midnight-system/` | System transactions |
| `pallet-cnight-observation` | `pallets/cnight-observation/` | Cardano bridge |
| `pallet-cnight-observation-mock` | `pallets/cnight-observation/mock/` | Test mock |
| `pallet-version` | `pallets/version/` | Version logging |
| `pallet-federated-authority` | `pallets/federated-authority/` | Multi-body governance |
| `pallet-federated-authority-observation` | `pallets/federated-authority-observation/` | Governance observation |
| `midnight-primitives` | `primitives/midnight/` | Core primitives |
| `midnight-primitives-ledger` | `primitives/ledger/` | Ledger primitives |
| `midnight-primitives-cnight-observation` | `primitives/cnight-observation/` | cNIGHT types |
| `midnight-primitives-federated-authority-observation` | `primitives/federated-authority-observation/` | Governance types |
| `midnight-primitives-mainchain-follower` | `primitives/mainchain-follower/` | Mainchain types |
| `midnight-toolkit` | `util/toolkit/` | CLI toolkit |
| `upgrader` | `util/upgrader/` | Upgrade service |
| `documented` | `util/documented/` | Doc macro |
| `documented_types` | `util/documented/documented_types/` | Doc types |
| `documented_proc_macro` | `util/documented/documented_proc_macro/` | Doc macro impl |
| `midnight-beefy-relay` | `relay/` | BEEFY relay |
| `e2e` | `tests/e2e/` | E2E tests |
| `docs` | `docs/` | Doc tests |

### Non-Rust Components

| Component | Path | Description |
|-----------|------|-------------|
| Local Environment | `local-environment/` | Docker network tools (TypeScript) |
| UI | `ui/` | React frontend |
| Toolkit JS | `util/toolkit-js/` | JavaScript CLI |

---

## See Also

- [CONTRIBUTING.md](CONTRIBUTING.md) - Contribution guidelines
- [CHANGELOG.md](CHANGELOG.md) - Version history
- [docs/development-workflow.md](docs/development-workflow.md) - Development workflow
- [docs/chain_specs.md](docs/chain_specs.md) - Chain specification documentation

