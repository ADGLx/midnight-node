# midnight-node-res

Static resources, network definitions, and configuration management for the Midnight blockchain node.

## Overview

This crate provides all static data required to bootstrap and configure Midnight networks:

- **[Genesis](https://docs.polkadot.com/polkadot-protocol/glossary/#genesis-configuration) data** - Pre-built genesis blocks and ledger states for each network
- **Network definitions** - Configuration for local, testnet, and production environments
- **Chain specifications** - Substrate chain specs for each network
- **Configuration files** - TOML-based node configuration
- **Test fixtures** - Sample transactions and contract data for testing

The crate uses `include_bytes!` to embed genesis data at compile time, ensuring reproducible builds across environments.

## API Specification

### Public Functions

- [**`default_cfg`**](https://github.com/midnightntwrk/midnight-node/blob/main/res/src/lib.rs#L35) - Load `default.toml` configuration
- [**`list_configs`**](https://github.com/midnightntwrk/midnight-node/blob/main/res/src/lib.rs#L42) - List available config presets
- [**`get_config`**](https://github.com/midnightntwrk/midnight-node/blob/main/res/src/lib.rs#L59) - Load a specific config by name
- [**`serialize_mn`**](https://github.com/midnightntwrk/midnight-node/blob/main/res/src/lib.rs#L94) - Serialize ledger types to bytes
- [**`deserialize_mn`**](https://github.com/midnightntwrk/midnight-node/blob/main/res/src/lib.rs#L102) - Deserialize ledger types from bytes

### Public Types (with `chain-spec` feature)

- [**`MidnightNetwork`**](https://github.com/midnightntwrk/midnight-node/blob/main/res/src/networks/mod.rs#L148) - Trait defining network configuration interface
- [**`UndeployedNetwork`**](https://github.com/midnightntwrk/midnight-node/blob/main/res/src/networks/definitions.rs#L20) - Local development network (Alice as authority)
- [**`CustomNetwork`**](https://github.com/midnightntwrk/midnight-node/blob/main/res/src/networks/definitions.rs#L78) - Runtime-configurable network definition
- [**`InitialAuthorityData`**](https://github.com/midnightntwrk/midnight-node/blob/main/res/src/networks/mod.rs#L37) - Validator public keys (aura, grandpa, crosschain, beefy)
- [**`MainChainScripts`**](https://github.com/midnightntwrk/midnight-node/blob/main/res/src/networks/mod.rs#L89) - Cardano script addresses and policy IDs
- [**`EndowedAccount`**](https://github.com/midnightntwrk/midnight-node/blob/main/res/src/networks/mod.rs#L83) - [Genesis](https://docs.polkadot.com/polkadot-protocol/glossary/#genesis-configuration) account with initial balance

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

- [`midnight-node`](https://github.com/midnightntwrk/midnight-node/blob/main/node/src/chain_spec/mod.rs) - Chain spec generation
- [`midnight-node-toolkit`](https://github.com/midnightntwrk/midnight-node/blob/main/util/toolkit/src/lib.rs) - Genesis generation and testing
- [`tests/e2e`](https://github.com/midnightntwrk/midnight-node/blob/main/tests/e2e/src/lib.rs) - Integration test fixtures

## Configuration Root Override

The `CFG_ROOT` static allows overriding the config directory at runtime for testing or custom deployments.

## Testing

```bash
# Run tests (requires test feature)
cargo test -p midnight-node-res --features test
```

## See Also

- [runtime](../runtime/README.md) - [Runtime](https://docs.polkadot.com/polkadot-protocol/glossary/#runtime) that uses these resources
- [Chain Specifications](../docs/chain_specs.md) - [Chain specification](https://docs.polkadot.com/polkadot-protocol/glossary/#chain-specification) documentation
- [node](../node/README.md) - Node that loads these resources

