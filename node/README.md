# midnight-node

Midnight blockchain node executable.

## Overview

This is the main entry point for running a Midnight node. It integrates:

- **Consensus** - [AURA](../GLOSSARY.md#aura-authority-round) (block production), [GRANDPA](../GLOSSARY.md#grandpa-ghost-based-recursive-ancestor-deriving-prefix-agreement) (finality), [BEEFY](../GLOSSARY.md#beefy-bridge-efficiency-enabling-finality-yielder) (bridge)
- **[Runtime](../GLOSSARY.md#runtime)** - `midnight-node-runtime` WASM execution
- **RPC** - JSON-RPC endpoints for state queries and transactions
- **Data Sources** - [db-sync](../GLOSSARY.md#db-sync) PostgreSQL for Cardano observations
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

```toml
[node]
name = "my-node"
chain = "qanet"

[rpc]
port = 9944
cors = ["*"]

[network]
bootnodes = ["/dns/bootnode.example.com/tcp/30333/p2p/..."]
```

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
| (WASM)           |     | (RocksDB)        |
+------------------+     +------------------+
```

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

- [runtime](../runtime/README.md) - [Runtime](../GLOSSARY.md#runtime) logic
- [Chain Specs](chain/README.md) - Chain specification details
- [docs/chain_specs.md](../docs/chain_specs.md) - [Chain spec](../GLOSSARY.md#chain-spec--chain-specification) documentation

