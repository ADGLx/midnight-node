# midnight-primitives-ledger

Ledger metrics and storage configuration primitives.

## Overview

This crate provides infrastructure for ledger performance monitoring and storage configuration:

- **Prometheus metrics** for transaction processing, validation, and storage operations
- **Externality extensions** to pass metrics and storage config to host functions
- **Hybrid histogram buckets** for precise timing measurements

## API Specification

### Public Types

| Type | Description |
|------|-------------|
| `LedgerMetrics` | Prometheus metric collectors |
| `LedgerMetricsExt` | Externality extension for metrics |
| `LedgerStorage` | Storage path and cache configuration |
| `LedgerStorageExt` | Externality extension for storage config |

### LedgerMetricsExt Methods

| Method | Description |
|--------|-------------|
| `observe_txs_processing_time` | Record transaction processing duration |
| `observe_system_txs_processing_time` | Record system tx duration |
| `observe_txs_validating_time` | Record validation duration |
| `observe_txs_size` | Record transaction size |
| `observe_storage_fetch_time` | Record state fetch duration |
| `observe_storage_flush_time` | Record state persist duration |

### Cargo Features

| Feature | Default | Description |
|---------|---------|-------------|
| `std` | Yes | Standard library support |

## Histogram Buckets

The crate uses hybrid linear+exponential buckets for precise measurements.

Constants from `primitives/ledger/src/lib.rs` (L41-L52):

**Time buckets:**
| Constant | Value | Description |
|----------|-------|-------------|
| `TIME_INTERVAL_LINEAR` | 0.05 (50ms) | Linear step size |
| `TIME_MAX_LINEAR` | 1.0 (1s) | Switch to exponential |
| `TIME_INCREASE_EXP` | 1.5 | Exponential growth factor |
| `TIME_MAX_EXP` | 60.0 (1min) | Maximum bucket |

**Size buckets:**
| Constant | Value | Description |
|----------|-------|-------------|
| `SIZE_INTERVAL_LINEAR` | 10 KiB | Linear step size |
| `SIZE_MAX_LINEAR` | 200 KiB | Switch to exponential |
| `SIZE_INCREASE_EXP` | 1.5 | Exponential growth factor |
| `SIZE_MAX_EXP` | 5 MiB | Maximum bucket |

## Integration

### Dependencies

- `prometheus-endpoint` - Prometheus metrics
- `sp-externalities` - Runtime extensions

### Used By

- `midnight-node` - Metric registration
- `midnight-node-ledger` - [Host function](https://docs.midnight.network/learn/glossary#host-function) metrics

## See Also

- [ledger](../../ledger/README.md) - Ledger bridge using these primitives

