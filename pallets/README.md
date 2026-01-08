# Pallets

Custom FRAME pallets implementing Midnight-specific blockchain logic.

## Overview

This directory contains the custom FRAME pallets that implement Midnight's blockchain logic. The pallets are divided into two categories: core pallets handling ledger operations, transaction processing, cross-chain observation, and version tracking; and governance pallets managing federated authority membership and mainchain synchronization. Together, these pallets enable privacy-preserving smart contract execution, cNIGHT to DUST bridging, and multi-body governance coordination.

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

## See Also

- [runtime/](../runtime/README.md) - Runtime that wires these pallets together
- [primitives/](../primitives/README.md) - Shared types used by pallets
- [Glossary](https://docs.midnight.network/learn/glossary) - Domain-specific terminology

