# midnight-primitives-ledger

Ledger metrics and storage configuration primitives.

## Overview

This crate provides infrastructure for ledger performance monitoring and storage configuration:

- **Prometheus metrics** for transaction processing, validation, and storage operations
- **Externality extensions** to pass metrics and storage config to host functions
- **Hybrid histogram buckets** for precise timing measurements

## API Specification

### Public Types

- [**`LedgerMetrics`**](https://github.com/midnightntwrk/midnight-node/blob/main/primitives/ledger/src/lib.rs#L25) - Prometheus metric collectors
- [**`LedgerStorage`**](https://github.com/midnightntwrk/midnight-node/blob/main/primitives/ledger/src/lib.rs#L222) - Storage path and cache configuration

### LedgerMetricsExt Methods

- [**`observe_txs_processing_time`**](https://github.com/midnightntwrk/midnight-node/blob/main/primitives/ledger/src/lib.rs#L189) - Record transaction processing duration
- [**`observe_system_txs_processing_time`**](https://github.com/midnightntwrk/midnight-node/blob/main/primitives/ledger/src/lib.rs#L183) - Record system tx duration
- [**`observe_txs_validating_time`**](https://github.com/midnightntwrk/midnight-node/blob/main/primitives/ledger/src/lib.rs#L195) - Record validation duration
- [**`observe_txs_size`**](https://github.com/midnightntwrk/midnight-node/blob/main/primitives/ledger/src/lib.rs#L201) - Record transaction size
- [**`observe_storage_fetch_time`**](https://github.com/midnightntwrk/midnight-node/blob/main/primitives/ledger/src/lib.rs#L207) - Record state fetch duration
- [**`observe_storage_flush_time`**](https://github.com/midnightntwrk/midnight-node/blob/main/primitives/ledger/src/lib.rs#L213) - Record state persist duration

## Histogram Buckets

The crate uses hybrid linear+exponential buckets for precise measurements.

Constants from `primitives/ledger/src/lib.rs` (L41-L52):

**Time buckets:**

- [**`TIME_INTERVAL_LINEAR`**](https://github.com/midnightntwrk/midnight-node/blob/main/primitives/ledger/src/lib.rs#L41) - 0.05 (50ms) - Linear step size
- [**`TIME_MAX_LINEAR`**](https://github.com/midnightntwrk/midnight-node/blob/main/primitives/ledger/src/lib.rs#L42) - 1.0 (1s) - Switch to exponential
- [**`TIME_INCREASE_EXP`**](https://github.com/midnightntwrk/midnight-node/blob/main/primitives/ledger/src/lib.rs#L43) - 1.5 - Exponential growth factor
- [**`TIME_MAX_EXP`**](https://github.com/midnightntwrk/midnight-node/blob/main/primitives/ledger/src/lib.rs#L44) - 60.0 (1min) - Maximum bucket

**Size buckets:**

- [**`SIZE_INTERVAL_LINEAR`**](https://github.com/midnightntwrk/midnight-node/blob/main/primitives/ledger/src/lib.rs#L49) - 10 KiB - Linear step size
- [**`SIZE_MAX_LINEAR`**](https://github.com/midnightntwrk/midnight-node/blob/main/primitives/ledger/src/lib.rs#L50) - 200 KiB - Switch to exponential
- [**`SIZE_INCREASE_EXP`**](https://github.com/midnightntwrk/midnight-node/blob/main/primitives/ledger/src/lib.rs#L51) - 1.5 - Exponential growth factor
- [**`SIZE_MAX_EXP`**](https://github.com/midnightntwrk/midnight-node/blob/main/primitives/ledger/src/lib.rs#L52) - 5 MiB - Maximum bucket

## Integration

### Dependencies

- `prometheus-endpoint` - Prometheus metrics
- `sp-externalities` - Runtime extensions

### Used By

- [`midnight-node`](https://github.com/midnightntwrk/midnight-node/blob/main/node/src/service.rs) - Metric registration
- [`midnight-node-ledger`](https://github.com/midnightntwrk/midnight-node/blob/main/ledger/src/lib.rs) - [Host function](https://docs.midnight.network/learn/glossary#host-function) metrics

## See Also

- [ledger](../../ledger/README.md) - Ledger bridge using these primitives

