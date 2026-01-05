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

### LedgerMetrics

```rust
pub struct LedgerMetrics {
    pub txs_processing_time: HistogramVec,      // Transaction execution time
    pub system_txs_processing_time: HistogramVec, // System tx time
    pub txs_validating_time: HistogramVec,      // Validation time
    pub txs_size: HistogramVec,                 // Transaction size
    pub storage_fetch_time: HistogramVec,       // State fetch time
    pub storage_flush_time: HistogramVec,       // State persist time
}
```

### LedgerMetricsExt Methods

| Method | Description |
|--------|-------------|
| `observe_txs_processing_time` | Record transaction processing duration |
| `observe_system_txs_processing_time` | Record system tx duration |
| `observe_txs_validating_time` | Record validation duration |
| `observe_txs_size` | Record transaction size |
| `observe_storage_fetch_time` | Record state fetch duration |
| `observe_storage_flush_time` | Record state persist duration |

### LedgerStorage

```rust
pub struct LedgerStorage {
    pub db_path: PathBuf,   // Path to ledger database
    pub cache_size: usize,  // Cache size in bytes
}
```

### Cargo Features

| Feature | Default | Description |
|---------|---------|-------------|
| `std` | Yes | Standard library support |

## Usage

### Registering Metrics

```rust
use midnight_primitives_ledger::LedgerMetrics;

let metrics = LedgerMetrics::register(&prometheus_registry)?;
```

### Using in Host Functions

```rust
use midnight_primitives_ledger::{LedgerMetricsExt, LedgerStorageExt};
use sp_externalities::set_and_run_with_externalities;

// Metrics are accessed via externalities in WASM host functions
externalities.register_extension(LedgerMetricsExt::new(metrics.clone()));
externalities.register_extension(LedgerStorageExt::new(storage_config));
```

### Observing Metrics

```rust
fn process_transaction(ext: &mut LedgerMetricsExt, tx: &[u8]) {
    let start = Instant::now();
    // ... process ...
    ext.observe_txs_processing_time(start.elapsed().as_secs_f64(), "standard");
    ext.observe_txs_size(tx.len() as f64, "standard");
}
```

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

