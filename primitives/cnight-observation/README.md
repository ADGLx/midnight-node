# midnight-primitives-cnight-observation

Shared types for [cNIGHT](https://docs.midnight.network/learn/glossary#cnight) token observation between Cardano and Midnight.

## Overview

This crate defines types for tracking [cNIGHT](https://docs.midnight.network/learn/glossary#cnight) token movements on Cardano:

- **Position tracking** - `CardanoPosition` for sync state
- **UTXO observation** - Types for registrations, redemptions, and token transfers
- **Runtime API** - Queries for observation configuration
- **Inherent types** - Data passed via block inherents

## API Specification

### Core Types

- **`CardanoPosition`** - Block hash, number, timestamp, and tx index
- **`ObservedUtxo`** - Header + data for an observed UTXO
- **`ObservedUtxoHeader`** - Position and UTXO identification
- **`ObservedUtxoData`** - Enum of observation types

### ObservedUtxoData Variants

- **`Registration`** - New Cardano-to-DUST wallet mapping
- **`Deregistration`** - Wallet mapping removal
- **`RedemptionCreate`** - Glacier Drop claim created
- **`RedemptionSpend`** - Glacier Drop claim spent
- **`AssetCreate`** - cNIGHT UTXO created
- **`AssetSpend`** - cNIGHT UTXO spent

### Address Types

- **`CardanoRewardAddressBytes`** - 29 bytes - Cardano stake/reward address
- **`DustPublicKeyBytes`** - 33 bytes - Compressed ECDSA public key

### Inherent

- **`ntobsrve`** - `MidnightObservationTokenMovement`

## Integration

### Dependencies

- `sidechain-domain` - `McBlockHash`, `McTxHash`
- `sp-api` / `sp-inherents` - Runtime API and inherent support

### Used By

- `pallet-cnight-observation` - Inherent processing
- `midnight-node` - Data source queries
- `partner-chains-db-sync-data-sources` - PostgreSQL queries

## See Also

- [pallet-cnight-observation](../../pallets/cnight-observation/README.md) - [Pallet](https://docs.polkadot.com/polkadot-protocol/glossary/#pallet) implementation
- [primitives-mainchain-follower](../mainchain-follower/README.md) - Data source traits

