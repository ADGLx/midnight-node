# midnight-node

Midnight blockchain node executable.

## Overview

This is the main entry point for running a Midnight node. It integrates:

- **Consensus** - [AURA](https://docs.midnight.network/learn/glossary#aura-authority-round) (block production), [GRANDPA](https://docs.midnight.network/learn/glossary#grandpa-ghost-based-recursive-ancestor-deriving-prefix-agreement) (finality), [BEEFY](https://docs.midnight.network/learn/glossary#beefy-bridge-efficiency-enabling-finality-yielder) (bridge)
- **[Runtime](https://docs.midnight.network/learn/glossary#runtime)** - `midnight-node-runtime` WASM execution
- **RPC** - JSON-RPC endpoints for state queries and transactions
- **Data Sources** - [db-sync](https://docs.midnight.network/learn/glossary#db-sync) PostgreSQL for Cardano observations
- **CLI** - Configuration and operational commands

## Installation

### From Source

```bash
cargo build --release -p midnight-node
./target/release/midnight-node --help
```

### Docker

```bash
docker run midnightntwrk/midnight-node:latest --help
```

## Usage

### Running a Node

```bash
# Development mode (single node)
midnight-node --dev

# Connect to testnet
midnight-node --chain qanet

# Custom chain spec
midnight-node --chain ./my-chain-spec.json

# With Cardano observations
midnight-node \
  --chain qanet \
  --db-sync-postgres-url "postgres://user:pass@localhost/cexplorer"
```

### Common Options

| Option | Description |
|--------|-------------|
| `--chain <SPEC>` | Chain spec (dev, qanet, or file path) |
| `--base-path <PATH>` | Data directory |
| `--name <NAME>` | Node name for telemetry |
| `--validator` | Enable block production |
| `--rpc-cors <ORIGINS>` | CORS for RPC (default: all) |
| `--rpc-port <PORT>` | RPC port (default: 9944) |
| `--db-sync-postgres-url` | Cardano db-sync connection |

### Subcommands

| Command | Description |
|---------|-------------|
| `key` | Key management (generate, inspect) |
| `build-spec` | Generate chain specification |
| `export-genesis-state` | Export genesis state |
| `export-genesis-wasm` | Export genesis WASM |
| `benchmark` | Runtime benchmarking |
| `try-runtime` | Test runtime upgrades |

## RPC Endpoints

### Midnight-Specific

| Method | Description |
|--------|-------------|
| `midnight_contractState` | Get contract state |
| `midnight_zswapStateRoot` | Get ZSwap root |
| `midnight_ledgerVersion` | Get ledger version |

### Substrate Standard

- `author_*` - Transaction submission
- `chain_*` - Block queries
- `state_*` - Storage queries
- `system_*` - Node info

## Configuration

Configuration can be provided via:

1. **CLI arguments** - `--option value`
2. **Environment variables** - `OPTION=value`
3. **Config file** - `--config config.toml`

### Example Config

Based on `res/cfg/default.toml`:

```toml
# Node behavior
wipe_chain_state = false
use_main_chain_follower_mock = false
validator = false

# Mainchain epoch configuration
mc__first_epoch_timestamp_millis = 1666656000000
mc__first_epoch_number = 0
mc__epoch_duration_millis = 86400000
mc__first_slot_number = 0
mc__slot_duration_millis = 1000

# Cardano parameters
cardano_security_parameter = 432
cardano_active_slots_coeff = 0.05
block_stability_margin = 10

# Storage
storage_cache_size = 0
trie_cache_size = 0

# CLI arguments (passed to Substrate)
argv = []
bootnodes = []
```

See `res/cfg/*.toml` for network-specific presets (dev, qanet, preview).

## Architecture

```
+------------------+     +------------------+     +------------------+
| CLI / Config     | --> | Service Builder  | --> | Node Service     |
+------------------+     +------------------+     +------------------+
                                                          |
        +------------------+------------------+------------+
        |                  |                  |
        v                  v                  v
+-------------+   +---------------+   +------------------+
| Consensus   |   | RPC Server    |   | Network          |
| AURA/GRANDPA|   | jsonrpsee     |   | libp2p           |
+-------------+   +---------------+   +------------------+
        |
        v
+------------------+     +------------------+
| Runtime          | --> | Ledger Storage   |
| (WASM)           |     | (ParityDB)       |
+------------------+     +------------------+
```

**Sources**: Service builder [`node/src/service.rs#L209-L283`](https://github.com/midnightntwrk/midnight-node/blob/main/node/src/service.rs#L209-L283), Ledger init [`node/src/service.rs#L217-L223`](https://github.com/midnightntwrk/midnight-node/blob/main/node/src/service.rs#L217-L223), Consensus [`node/src/service.rs#L327-L360`](https://github.com/midnightntwrk/midnight-node/blob/main/node/src/service.rs#L327-L360)

## Cargo Features

| Feature | Default | Description |
|---------|---------|-------------|
| (none) | Yes | Standard build |
| `runtime-benchmarks` | No | Include benchmarking |
| `try-runtime` | No | Include try-runtime |
| `experimental` | No | Experimental features |

## Development

### Building

```bash
# Debug build
cargo build -p midnight-node

# Release build
cargo build --release -p midnight-node

# With benchmarks
cargo build --release -p midnight-node --features runtime-benchmarks
```

### Running Tests

```bash
cargo test -p midnight-node
```

## See Also

- [runtime](../runtime/README.md) - [Runtime](https://docs.midnight.network/learn/glossary#runtime) logic
- [Chain Specs](chain/readme.md) - Chain specification details
- [docs/chain_specs.md](../docs/chain_specs.md) - [Chain spec](https://docs.midnight.network/learn/glossary#chain-spec--chain-specification) documentation

