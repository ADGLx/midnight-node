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

| Type | Description |
|------|-------------|
| `ObservedUtxo` | Complete observed UTXO with header and data |
| `ObservedUtxoHeader` | Position and identification |
| `ObservedUtxoData` | Enum of observation types |
| `RegistrationData` | Registration details |
| `DeregistrationData` | Deregistration details |
| `CreateData` | UTXO creation details |
| `SpendData` | UTXO spend details |
| `RedemptionCreateData` | Glacier Drop create |
| `RedemptionSpendData` | Glacier Drop spend |

### Local Type

| Type | Description |
|------|-------------|
| `MidnightObservationTokenMovement` | Batch of observed UTXOs with next position |

## Integration

### Dependencies

- `midnight-primitives-cnight-observation` - Core observation types

### Used By

- `pallet-cnight-observation` - UTXO processing
- `partner-chains-db-sync-data-sources` - PostgreSQL queries
- `partner-chains-mock-data-sources` - Test data

## See Also

- [primitives-cnight-observation](../cnight-observation/README.md) - Core types
- [pallet-cnight-observation](../../pallets/cnight-observation/README.md) - Consumer
