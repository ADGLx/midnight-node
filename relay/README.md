# midnight-beefy-relay

[BEEFY](https://docs.midnight.network/learn/glossary#beefy-bridge-efficiency-enabling-finality-yielder) key management and relay service for cross-chain bridge operations.

## Overview

This crate provides tooling for managing [BEEFY](https://docs.midnight.network/learn/glossary#beefy-bridge-efficiency-enabling-finality-yielder) consensus keys required for Midnight's cross-chain bridge with Cardano. [BEEFY](https://docs.midnight.network/learn/glossary#beefy-bridge-efficiency-enabling-finality-yielder) produces compact finality proofs that can be efficiently verified on external chains.

## Prerequisites

Start the Midnight node with archival settings:

```bash
midnight-node \
    --state-pruning archive \
    --blocks-pruning archive \
    --enable-offchain-indexing true
```

Ensure [BEEFY](https://docs.midnight.network/learn/glossary#beefy-bridge-efficiency-enabling-finality-yielder) keys are inserted and the first session has passed.

## API Specification

### CLI Commands

| Command | Description |
|---------|-------------|
| `midnight-beefy-relay --keys-path <file>` | Insert [BEEFY](https://docs.midnight.network/learn/glossary#beefy-bridge-efficiency-enabling-finality-yielder) keys from JSON file |

### Key File Format

```json
[
  {
    "suri": "//Alice",
    "pub_key": "0x020a1091341fe5664bfa1782d5e04779689068c916b04cb365ec3153755684d9a1",
    "node_url": "ws://localhost:9937"
  }
]
```

| Field | Description |
|-------|-------------|
| `suri` | Secret URI (seed phrase or dev account) |
| `pub_key` | ECDSA public key in hex format |
| `node_url` | WebSocket URL of the target node |

### Cargo Features

| Feature | Default | Description |
|---------|---------|-------------|
| `std` | Yes | Standard library support |

## Usage

### Inserting BEEFY Keys

#### Via this Relayer

```bash
cargo run --bin midnight-beefy-relay -- --keys-path=./beefy-keys.json
```

Output:
```
Added beefy key: 0x020a1091341fe5664bfa1782d5e04779689068c916b04cb365ec3153755684d9a1 to ws://localhost:9933
```

#### Via Polkadot.js

1. Go to Developer → RPC Calls
2. Select `author` endpoint and `insertKey` method
3. Input: keyType=`beef`, suri=`<secret>`, publicKey=`<ECDSA pubkey>`

#### Via curl

```bash
curl http://localhost:9933 -H "Content-Type:application/json;charset=utf-8" -d '{
    "jsonrpc":"2.0",
    "id":1,
    "method":"author_insertKey",
    "params":["beef","<suri>","<publicKey>"]
}'
```

### Example Key Files

See `res/mock-bridge-data/beefy-keys-mock.json` for example configuration.

## Architecture

```
+------------------+     +------------------+     +------------------+
| BEEFY Key JSON   | --> | midnight-beefy-  | --> | Node RPC         |
| (keys + URLs)    |     | relay            |     | author_insertKey |
+------------------+     +------------------+     +------------------+
                                                          |
                                                          v
                                                  +------------------+
                                                  | BEEFY Consensus  |
                                                  | Key Storage      |
                                                  +------------------+
```

**Sources**: [`relay/src/main.rs`](https://github.com/midnightntwrk/midnight-node/blob/main/relay/src/main.rs) - CLI entry, [`relay/src/beefy_keys.rs#L37`](https://github.com/midnightntwrk/midnight-node/blob/main/relay/src/beefy_keys.rs#L37) - `insert_key`

## Integration

### Dependencies

- Midnight node with [BEEFY](https://docs.midnight.network/learn/glossary#beefy-bridge-efficiency-enabling-finality-yielder) enabled
- ECDSA key pairs for each validator

### Used By

- Validator operators for key management
- CI/CD for test network setup

## Testing

```bash
cargo test -p relay
```

## See Also

- [GLOSSARY - BEEFY](https://docs.midnight.network/learn/glossary#beefy-bridge-efficiency-enabling-finality-yielder) - Protocol description
- [res/mock-bridge-data](../res/mock-bridge-data/) - Example key files
- [node](../node/README.md) - Node configuration
