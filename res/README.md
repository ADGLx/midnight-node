# midnight-node-res

Static resources, network definitions, and configuration management for the Midnight blockchain node.

## Overview

This crate provides all static data required to bootstrap and configure Midnight networks:

- **[Genesis](https://docs.midnight.network/learn/glossary#genesis) data** - Pre-built genesis blocks and ledger states for each network
- **Network definitions** - Configuration for local, testnet, and production environments
- **Chain specifications** - Substrate chain specs for each network
- **Configuration files** - TOML-based node configuration
- **Test fixtures** - Sample transactions and contract data for testing

The crate uses `include_bytes!` to embed genesis data at compile time, ensuring reproducible builds across environments.

## API Specification

### Public Functions

| Function | Signature | Description |
|----------|-----------|-------------|
| `default_cfg` | `fn() -> String` | Load `default.toml` configuration |
| `list_configs` | `fn() -> Vec<String>` | List available config presets |
| `get_config` | `fn(name: &str) -> Option<String>` | Load a specific config by name |
| `serialize_mn` | `fn<T>(value: &T) -> Result<Vec<u8>, Error>` | Serialize ledger types to bytes |
| `deserialize_mn` | `fn<T, H>(bytes: H) -> Result<T, Error>` | Deserialize ledger types from bytes |

### Public Types (with `chain-spec` feature)

| Type | Description |
|------|-------------|
| `MidnightNetwork` | Trait defining network configuration interface |
| `UndeployedNetwork` | Local development network (Alice as authority) |
| `CustomNetwork` | Runtime-configurable network definition |
| `InitialAuthorityData` | Validator public keys (aura, grandpa, crosschain, beefy) |
| `MainChainScripts` | Cardano script addresses and policy IDs |
| `EndowedAccount` | [Genesis](https://docs.midnight.network/learn/glossary#genesis) account with initial balance |

### Cargo Features

| Feature | Default | Description |
|---------|---------|-------------|
| `std` | Yes | Standard library support |
| `chain-spec` | No | Network definitions and chain spec builders |
| `test` | No | Test transaction fixtures |
| `runtime-benchmarks` | No | Benchmark-specific resources |

## Directory Structure

```
res/
├── cfg/                    # Node configuration presets
│   ├── default.toml        # Default configuration
│   ├── dev.toml            # Development settings
│   └── qanet.toml          # QA network settings
├── genesis/                # Genesis data (compiled into binary)
│   ├── genesis_block_*.mn  # Serialized genesis blocks
│   └── genesis_state_*.mn  # Serialized ledger states
├── dev/                    # Local development configs
├── qanet/                  # QA network chain specs
├── preview/                # Preview network chain specs
├── node-dev-01/            # Dev node configurations
├── perfnet/                # Performance testing network
├── mock-bridge-data/       # Mock Cardano bridge data
├── test-contract/          # Test contract transactions
├── test-zswap/             # ZSwap test transactions
└── test-claim-mint/        # Claim mint test data
```

## Available Networks

| Network | Chain Type | Description |
|---------|------------|-------------|
| `undeployed` | Local | Single-node development (Alice) |
| `node-dev-01` | Development | Multi-node local development |
| `qanet` | Live | QA testing network |
| `preview` | Live | Preview/staging network |
| `perfnet` | Live | Performance testing |

## Integration

### Dependencies

- `midnight-serialize` - Ledger type serialization
- `serde` / `serde_json` - Configuration parsing
- `sp-core` (optional) - Cryptographic types
- `sc-service` (optional) - Chain type definitions

### Used By

- `midnight-node` - Chain spec generation
- `midnight-node-toolkit` - Genesis generation and testing
- `tests/e2e` - Integration test fixtures

## Configuration Root Override

The `CFG_ROOT` static allows overriding the config directory at runtime for testing or custom deployments.

## Testing

```bash
# Run tests (requires test feature)
cargo test -p midnight-node-res --features test
```

## See Also

- [runtime](../runtime/README.md) - [Runtime](https://docs.midnight.network/learn/glossary#runtime) that uses these resources
- [Chain Specifications](../docs/chain_specs.md) - [Chain spec](https://docs.midnight.network/learn/glossary#chain-spec--chain-specification) documentation
- [node](../node/README.md) - Node that loads these resources

