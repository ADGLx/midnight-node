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

| Type | Description |
|------|-------------|
| `CardanoPosition` | Block hash, number, timestamp, and tx index |
| `ObservedUtxo` | Header + data for an observed UTXO |
| `ObservedUtxoHeader` | Position and UTXO identification |
| `ObservedUtxoData` | Enum of observation types |

### CardanoPosition

```rust
pub struct CardanoPosition {
    pub block_hash: McBlockHash,
    pub block_number: u32,
    pub block_timestamp: TimestampUnixMillis,
    pub tx_index_in_block: u32,
}
```

### ObservedUtxoData Variants

| Variant | Description |
|---------|-------------|
| `Registration` | New Cardano-to-DUST wallet mapping |
| `Deregistration` | Wallet mapping removal |
| `RedemptionCreate` | Glacier Drop claim created |
| `RedemptionSpend` | Glacier Drop claim spent |
| `AssetCreate` | cNIGHT UTXO created |
| `AssetSpend` | cNIGHT UTXO spent |

### Address Types

| Type | Size | Description |
|------|------|-------------|
| `CardanoRewardAddressBytes` | 29 bytes | Cardano stake/reward address |
| `DustPublicKeyBytes` | 33 bytes | Compressed ECDSA public key |

### CNightAddresses (Genesis Config)

```rust
pub struct CNightAddresses {
    pub mapping_validator_address: String,    // Bech32 address
    pub auth_token_asset_name: String,
    pub redemption_validator_address: String, // Bech32 address
    pub cnight_policy_id: [u8; 28],
    pub cnight_asset_name: String,
}
```

### Runtime API

```rust
pub trait CNightObservationApi {
    fn get_redemption_validator_address() -> Vec<u8>;
    fn get_mapping_validator_address() -> Vec<u8>;
    fn get_auth_token_asset_name() -> Vec<u8>;
    fn get_cnight_token_identifier() -> (Vec<u8>, Vec<u8>);
    fn get_next_cardano_position() -> CardanoPosition;
    fn get_cardano_block_window_size() -> u32;
    fn get_utxo_capacity_per_block() -> u32;
}
```

### Inherent

| Identifier | Type |
|------------|------|
| `ntobsrve` | `MidnightObservationTokenMovement` |

### Cargo Features

| Feature | Default | Description |
|---------|---------|-------------|
| `std` | Yes | Enables sqlx, thiserror, serde_valid |

## Usage

### Parsing Observed UTXOs

```rust
use midnight_primitives_cnight_observation::{ObservedUtxo, ObservedUtxoData};

for utxo in observed_utxos {
    match utxo.data {
        ObservedUtxoData::Registration(reg) => {
            // Process registration
        }
        ObservedUtxoData::AssetCreate(create) => {
            // Process cNIGHT UTXO creation
        }
        // ...
    }
}
```

### Position Comparison

```rust
// Positions are ordered by (block_number, tx_index)
if current_position < target_position {
    // More UTXOs to process
}
```

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

