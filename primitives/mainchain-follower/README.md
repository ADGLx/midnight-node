# midnight-primitives-mainchain-follower

Data source traits and types for following Cardano mainchain state.

## Overview

This crate defines the interface between Midnight node and Cardano data sources ([db-sync](https://docs.midnight.network/learn/glossary#db-sync) PostgreSQL or mock). It provides:

- **Observation types** - [UTXO](https://docs.midnight.network/learn/glossary#utxo-unspent-transaction-output) data structures matching Cardano observations
- **Re-exports** - Types from `midnight-primitives-cnight-observation` for convenience

## Compile-Time Checked Database Queries

The database queries in this repo are checked at compile-time. When changing a query, the metadata for that query must be re-generated. This can be done via earthly:

```bash
$ earthly +rebuild-sqlx
```

**NOTE:** `local-env` must be running for this to work! `earthly +start-local-env-latest`

## API Specification

### Re-exported Types

From `midnight-primitives-cnight-observation`:

- [**`ObservedUtxo`**](https://github.com/midnightntwrk/midnight-node/blob/main/primitives/cnight-observation/src/lib.rs#L241) - Complete observed UTXO with header and data
- [**`ObservedUtxoHeader`**](https://github.com/midnightntwrk/midnight-node/blob/main/primitives/cnight-observation/src/lib.rs#L346) - Position and identification
- [**`ObservedUtxoData`**](https://github.com/midnightntwrk/midnight-node/blob/main/primitives/cnight-observation/src/lib.rs#L273) - Enum of observation types
- [**`RegistrationData`**](https://github.com/midnightntwrk/midnight-node/blob/main/primitives/cnight-observation/src/lib.rs#L306) - Registration details
- [**`DeregistrationData`**](https://github.com/midnightntwrk/midnight-node/blob/main/primitives/cnight-observation/src/lib.rs#L314) - Deregistration details
- [**`CreateData`**](https://github.com/midnightntwrk/midnight-node/blob/main/primitives/cnight-observation/src/lib.rs#L322) - UTXO creation details
- [**`SpendData`**](https://github.com/midnightntwrk/midnight-node/blob/main/primitives/cnight-observation/src/lib.rs#L332) - UTXO spend details
- [**`RedemptionCreateData`**](https://github.com/midnightntwrk/midnight-node/blob/main/primitives/cnight-observation/src/lib.rs#L285) - Glacier Drop create
- [**`RedemptionSpendData`**](https://github.com/midnightntwrk/midnight-node/blob/main/primitives/cnight-observation/src/lib.rs#L295) - Glacier Drop spend

### Local Type

- [**`MidnightObservationTokenMovement`**](https://github.com/midnightntwrk/midnight-node/blob/main/primitives/mainchain-follower/src/lib.rs#L19) - Batch of observed UTXOs with next position

## Integration

### Dependencies

- [`midnight-primitives-cnight-observation`](https://github.com/midnightntwrk/midnight-node/blob/main/primitives/cnight-observation/src/lib.rs) - Core observation types

### Used By

- [`pallet-cnight-observation`](https://github.com/midnightntwrk/midnight-node/blob/main/pallets/cnight-observation/src/lib.rs) - UTXO processing
- `partner-chains-db-sync-data-sources` - PostgreSQL queries (external)
- `partner-chains-mock-data-sources` - Test data (external)

## See Also

- [primitives-cnight-observation](../cnight-observation/README.md) - Core types
- [pallet-cnight-observation](../../pallets/cnight-observation/README.md) - Consumer
