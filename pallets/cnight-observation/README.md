# pallet-cnight-observation

[Pallet](https://docs.midnight.network/learn/glossary#pallet) for observing [cNIGHT](https://docs.midnight.network/learn/glossary#cnight) token movements on Cardano and generating [DUST](https://docs.midnight.network/learn/glossary#dust) on Midnight.

## Overview

This pallet bridges Cardano mainchain observations to the Midnight ledger. It tracks:

- **[cNIGHT](https://docs.midnight.network/learn/glossary#cnight) registrations** - Wallet mappings between Cardano reward addresses and [DUST](https://docs.midnight.network/learn/glossary#dust) public keys
- **[cNIGHT](https://docs.midnight.network/learn/glossary#cnight) UTXOs** - Token holdings that generate [DUST](https://docs.midnight.network/learn/glossary#dust) over time
- **[Glacier Drop](https://docs.midnight.network/learn/glossary#glacier-drop) redemptions** - One-time token claims

Observations arrive via inherents from the mainchain follower data source. The pallet generates Cardano Midnight System Transactions (CMSTs) that are applied to the ledger state.

## API Specification

### Storage Items

| Name | Type | Description |
|------|------|-------------|
| `NextCardanoPosition` | `CardanoPosition` | Next block/tx to process |
| `MainChainRedemptionValidatorAddress` | `BoundedVec<u8>` | Glacier Drop contract address |
| `MainChainMappingValidatorAddress` | `BoundedVec<u8>` | Registration mapping contract |
| `CNightIdentifier` | `(PolicyId, AssetName)` | cNIGHT token identifier |
| `MainChainAuthTokenAssetName` | `BoundedVec<u8>` | Auth token asset name |
| `CardanoTxCapacityPerBlock` | `u32` | Max Cardano txs per block |
| `CardanoBlockWindowSize` | `u32` | Observation window size |

### Events

| Event | Description |
|-------|-------------|
| `Registration` | New Cardano-to-DUST wallet mapping |
| `Deregistration` | Wallet mapping removed |
| `MappingAdded` | UTXO mapping created |
| `MappingRemoved` | UTXO mapping spent |
| `SystemTransactionApplied` | CMST applied to ledger |

### Errors

| Error | Description |
|-------|-------------|
| `MaxCardanoAddrLengthExceeded` | Address too long (>108 chars) |
| `MaxRegistrationsExceeded` | Too many registrations in block |
| `LedgerApiError` | Ledger operation failed |

### Config Trait

| Associated Type | Description |
|-----------------|-------------|
| `MidnightSystemTransactionExecutor` | Interface to apply system transactions |

### Inherent

| Identifier | Data Type | Description |
|------------|-----------|-------------|
| `ntobsrve` | `MidnightObservationTokenMovement` | Observed UTXOs and next position |

### Cargo Features

| Feature | Default | Description |
|---------|---------|-------------|
| `std` | Yes | Standard library support |

## Architecture

```
Observation Flow:
+------------------+     +------------------+     +------------------+
| db-sync          | --> | Inherent Data    | --> | pallet-cnight-   |
| (PostgreSQL)     |     | Provider         |     | observation      |
+------------------+     +------------------+     +------------------+
                                                          |
                                                          v
                         +------------------+     +------------------+
                         | MidnightSystem:: | <-- | Generate CMST    |
                         | execute_system_tx|     | (Registration/   |
                         +------------------+     |  UTXO changes)   |
                                                  +------------------+

Data Types:
+------------------+     +------------------+
| ObservedUtxo     |     | CardanoPosition  |
|  - header        |     |  - block_hash    |
|  - data (enum)   |     |  - block_number  |
|    - Registration|     |  - tx_index      |
|    - Create/Spend|     +------------------+
+------------------+
```

**Sources**: [`primitives/mainchain-follower/src/idp/cnight_observation.rs#L30-L33`](https://github.com/midnightntwrk/midnight-node/blob/main/primitives/mainchain-follower/src/idp/cnight_observation.rs#L30-L33), [`node/src/inherent_data.rs#L141`](https://github.com/midnightntwrk/midnight-node/blob/main/node/src/inherent_data.rs#L141)

## Usage

### Runtime Configuration

```rust
impl pallet_cnight_observation::Config for Runtime {
    type MidnightSystemTransactionExecutor = MidnightSystem;
}
```

### Genesis Configuration

From `pallets/cnight-observation/src/config.rs` - genesis is configured via JSON:

```json
{
  "addresses": {
    "mapping_validator_address": "addr_test1...",
    "redemption_validator_address": "addr_test1...",
    "auth_token_asset_name": "",
    "cnight_policy_id": "03cf16101d110dcad9cacb225f0d1e63a8809979e7feb60426995414",
    "cnight_asset_name": ""
  },
  "observed_utxos": { "start": {...}, "end": {...}, "utxos": [] },
  "mappings": {},
  "utxo_owners": {},
  "next_cardano_position": { "block_hash": "...", "block_number": 0, ... },
  "system_tx": null
}
```

See `pallets/cnight-observation/src/config.rs` for full `CNightGenesis` structure.

## Integration

### Dependencies

- `midnight-primitives-cnight-observation` - Shared types
- `midnight-primitives-mainchain-follower` - Data source traits
- `pallet-midnight-system` - System transaction execution

### Used By

- `runtime` - Inherent provider
- `midnight-node` - Data source wiring

## Testing

```bash
cargo test -p pallet-cnight-observation
```

## See Also

- [pallet-cnight-observation-mock](mock/README.md) - Test mock runtime
- [primitives-cnight-observation](../../primitives/cnight-observation/README.md) - Shared types
- [primitives-mainchain-follower](../../primitives/mainchain-follower/README.md) - Data source

