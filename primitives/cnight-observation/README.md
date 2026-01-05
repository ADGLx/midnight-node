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

- [**`CardanoPosition`**](https://github.com/midnightntwrk/midnight-node/blob/main/primitives/cnight-observation/src/lib.rs#L30) - Block hash, number, timestamp, and tx index
- [**`ObservedUtxo`**](https://github.com/midnightntwrk/midnight-node/blob/main/primitives/cnight-observation/src/lib.rs#L50) - Header + data for an observed UTXO
- [**`ObservedUtxoHeader`**](https://github.com/midnightntwrk/midnight-node/blob/main/primitives/cnight-observation/src/lib.rs#L60) - Position and UTXO identification
- [**`ObservedUtxoData`**](https://github.com/midnightntwrk/midnight-node/blob/main/primitives/cnight-observation/src/lib.rs#L70) - Enum of observation types

### ObservedUtxoData Variants

- [**`Registration`**](https://github.com/midnightntwrk/midnight-node/blob/main/primitives/cnight-observation/src/lib.rs#L75) - New Cardano-to-DUST wallet mapping
- [**`Deregistration`**](https://github.com/midnightntwrk/midnight-node/blob/main/primitives/cnight-observation/src/lib.rs#L77) - Wallet mapping removal
- [**`RedemptionCreate`**](https://github.com/midnightntwrk/midnight-node/blob/main/primitives/cnight-observation/src/lib.rs#L79) - Glacier Drop claim created
- [**`RedemptionSpend`**](https://github.com/midnightntwrk/midnight-node/blob/main/primitives/cnight-observation/src/lib.rs#L81) - Glacier Drop claim spent
- [**`AssetCreate`**](https://github.com/midnightntwrk/midnight-node/blob/main/primitives/cnight-observation/src/lib.rs#L83) - cNIGHT UTXO created
- [**`AssetSpend`**](https://github.com/midnightntwrk/midnight-node/blob/main/primitives/cnight-observation/src/lib.rs#L85) - cNIGHT UTXO spent

### Address Types

- [**`CardanoRewardAddressBytes`**](https://github.com/midnightntwrk/midnight-node/blob/main/primitives/cnight-observation/src/lib.rs#L95) - Cardano stake/reward address (29 bytes)
- [**`DustPublicKeyBytes`**](https://github.com/midnightntwrk/midnight-node/blob/main/primitives/cnight-observation/src/lib.rs#L100) - Compressed ECDSA public key (33 bytes)

### Inherent

- [**`ntobsrve`**](https://github.com/midnightntwrk/midnight-node/blob/main/primitives/cnight-observation/src/lib.rs#L20) - Observed UTXOs and next position

## Integration

### Dependencies

- `sidechain-domain` - `McBlockHash`, `McTxHash`
- `sp-api` / `sp-inherents` - Runtime API and inherent support

### Used By

- `pallet-cnight-observation` - Inherent processing
- `midnight-node` - Data source queries
- `partner-chains-db-sync-data-sources` - PostgreSQL queries

## See Also

- [pallet-cnight-observation](../../pallets/cnight-observation/README.md) - [Pallet](https://docs.midnight.network/learn/glossary#pallet) implementation
- [primitives-mainchain-follower](../mainchain-follower/README.md) - Data source traits

