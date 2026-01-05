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

- **`--chain <SPEC>`** - Chain spec (dev, qanet, or file path)
- **`--base-path <PATH>`** - Data directory
- **`--name <NAME>`** - Node name for telemetry
- **`--validator`** - Enable block production
- **`--rpc-cors <ORIGINS>`** - CORS for RPC (default: all)
- **`--rpc-port <PORT>`** - RPC port (default: 9944)
- **`--db-sync-postgres-url`** - Cardano db-sync connection

### Subcommands

- **`key`** - Key management (generate, inspect)
- **`build-spec`** - Generate chain specification
- **`export-genesis-state`** - Export genesis state
- **`export-genesis-wasm`** - Export genesis WASM
- **`benchmark`** - Runtime benchmarking
- **`try-runtime`** - Test runtime upgrades

## RPC Endpoints

### Midnight-Specific

- **`midnight_contractState`** - Get contract state
- **`midnight_zswapStateRoot`** - Get ZSwap root
- **`midnight_ledgerVersion`** - Get ledger version

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

The node executable integrates multiple subsystems into a cohesive blockchain client. CLI arguments and configuration files are parsed and passed to the Service Builder, which constructs the Node Service with all required components. The service spawns parallel subsystems: AURA for block production, GRANDPA for finality, jsonrpsee for RPC handling, and libp2p for peer networking. The consensus layer drives the WASM runtime executor, which processes blocks and transactions against the Midnight ledger stored in ParityDB.

```mermaid
flowchart TB
    A[CLI / Config] --> B[Service Builder]
    B --> C[Node Service]
    C --> D[Consensus<br/>AURA/GRANDPA]
    C --> E[RPC Server<br/>jsonrpsee]
    C --> F[Network<br/>libp2p]
    D --> G[Runtime<br/>WASM]
    G --> H[Ledger Storage<br/>ParityDB]
```

**Sources**: [[1]](https://github.com/midnightntwrk/midnight-node/blob/main/node/src/service.rs#L209-L283) [[2]](https://github.com/midnightntwrk/midnight-node/blob/main/node/src/service.rs#L327-L360) [[3]](https://github.com/midnightntwrk/midnight-node/blob/main/node/src/command.rs#L50-L150)


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

