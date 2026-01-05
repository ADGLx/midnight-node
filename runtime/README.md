# Midnight Node Runtime

The WASM runtime that defines the Midnight blockchain's state transition function, consensus rules, and on-chain logic.

## Overview

This crate compiles to WebAssembly (WASM) and executes within the Substrate executor to process blocks and transactions. It composes [FRAME](https://docs.midnight.network/learn/glossary#frame-framework-for-runtime-aggregation-of-modularized-entities) pallets into a complete blockchain runtime, defining:

- **[Pallet](https://docs.midnight.network/learn/glossary#pallet) composition** - Which pallets are included and how they're configured
- **Consensus parameters** - [AURA](https://docs.midnight.network/learn/glossary#aura-authority-round) (6-second blocks), [GRANDPA](https://docs.midnight.network/learn/glossary#grandpa-ghost-based-recursive-ancestor-deriving-prefix-agreement) finality, [BEEFY](https://docs.midnight.network/learn/glossary#beefy-bridge-efficiency-enabling-finality-yielder) bridge support
- **Type definitions** - Account IDs, balances, block numbers, signatures
- **[Runtime](https://docs.midnight.network/learn/glossary#runtime) APIs** - Interfaces for RPC queries and off-chain interactions
- **Storage migrations** - Upgrade logic for runtime changes

The runtime is the "business logic" of the chain—validators execute it identically to reach consensus on state transitions.

## API Specification

### Core Types

| Type | Description |
|------|-------------|
| [`BlockNumber`](https://github.com/midnightntwrk/midnight-node/blob/main/runtime/src/lib.rs#L200) | Index to a block in the chain (`u32`) |
| [`AccountId`](https://github.com/midnightntwrk/midnight-node/blob/main/runtime/src/lib.rs#L206) | Account identifier derived from public key |
| [`Balance`](https://github.com/midnightntwrk/midnight-node/blob/main/runtime/src/lib.rs#L209) | Account balance type (`u128`) |
| [`Nonce`](https://github.com/midnightntwrk/midnight-node/blob/main/runtime/src/lib.rs#L203) | Transaction index for replay protection (`u32`) |
| [`Hash`](https://github.com/midnightntwrk/midnight-node/blob/main/runtime/src/lib.rs#L212) | 256-bit hash for blocks and tries (`sp_core::H256`) |
| [`Signature`](https://github.com/midnightntwrk/midnight-node/blob/main/runtime/src/lib.rs#L218) | Transaction signature (Sr25519/Ed25519/ECDSA) |

### Block Structure

| Type | Description |
|------|-------------|
| [`Block`](https://github.com/midnightntwrk/midnight-node/blob/main/runtime/src/lib.rs#L224) | Standard Substrate block containing header and extrinsics |
| [`Header`](https://github.com/midnightntwrk/midnight-node/blob/main/runtime/src/lib.rs#L221) | Block header with number, parent hash, state root, and Blake2-256 hashing |
| [`UncheckedExtrinsic`](https://github.com/midnightntwrk/midnight-node/blob/main/runtime/src/lib.rs#L250) | Transaction with sender address, call data, signature, and extensions |
| [`SignedExtra`](https://github.com/midnightntwrk/midnight-node/blob/main/runtime/src/lib.rs#L232) | Transaction extensions (nonce, mortality period, weight checks) |

### Runtime APIs

| API | Description |
|-----|-------------|
| `MidnightRuntimeApi` | Ledger state queries (contract state, transaction decoding, network ID) |
| `SessionValidatorManagementApi` | Committee queries (current/next validators) |
| `CNightObservationApi` | Cardano bridge configuration |
| `FederatedAuthorityObservationApi` | Governance address queries |
| `GovernedMapIDPApi` | Key-value governance map state |
| `AuraApi` | Block production slot duration and authorities |
| `GrandpaApi` | Finality authorities and set ID |
| `BeefyApi` | Bridge validator set and proofs |
| `MmrApi` | Merkle Mountain Range root and proofs |

### Cargo Features

| Feature | Default | Description |
|---------|---------|-------------|
| `std` | Yes | Standard library support (required for native execution) |
| `runtime-benchmarks` | No | Enables weight benchmarking |
| `try-runtime` | No | Enables try-runtime testing for migrations |
| `experimental` | No | Experimental features (block rewards) |

## Architecture

### Pallet Composition

```
+-----------------------------------------------------------------------+
|                              Runtime                                  |
+-----------------------------------------------------------------------+
| System Pallets                                                        |
|   System, Timestamp, Sudo, Scheduler, TxPause, Preimage, Migrations   |
+-----------------------------------------------------------------------+
| Consensus Pallets                                                     |
|   Aura, Grandpa, Beefy, Mmr, BeefyMmrLeaf                             |
+-----------------------------------------------------------------------+
| Midnight Pallets                                                      |
|   Midnight, MidnightSystem, CNightObservation, NodeVersion            |
+-----------------------------------------------------------------------+
| Governance Pallets                                                    |
|   Council, CouncilMembership, TechnicalCommittee,                     |
|   TechnicalCommitteeMembership, FederatedAuthority,                   |
|   FederatedAuthorityObservation                                       |
+-----------------------------------------------------------------------+
| Partner Chain Pallets                                                 |
|   Sidechain, Session, SessionCommitteeManagement, GovernedMap         |
+-----------------------------------------------------------------------+
```

**Sources**: [`runtime/src/lib.rs#L992-L1090`](https://github.com/midnightntwrk/midnight-node/blob/main/runtime/src/lib.rs#L992-L1090) - `#[frame_support::runtime]` pallet declarations

### Pallet Index Map

| Index | [Pallet](https://docs.midnight.network/learn/glossary#pallet) | Purpose |
|-------|--------|---------|
| 0 | System | Core frame system |
| 1 | Timestamp | Block timestamps |
| 2 | Aura | Block production |
| 3 | Grandpa | Finality |
| 5 | Midnight | Ledger state and transactions |
| 6 | MidnightSystem | System-level ledger operations |
| 7 | Sudo | Administrative operations |
| 11 | NodeVersion | [Runtime](https://docs.midnight.network/learn/glossary#runtime) version tracking |
| 13 | CNightObservation | [cNIGHT](https://docs.midnight.network/learn/glossary#cnight) token bridge |
| 21-23 | Beefy/Mmr | Bridge support |
| 40-45 | Governance | [Council](https://docs.midnight.network/learn/glossary#council), TC, [Federated Authority](https://docs.midnight.network/learn/glossary#federated-authority) |

### Consensus Configuration

Values from `runtime/src/lib.rs`:

| Parameter | Value | Source Line | Description |
|-----------|-------|-------------|-------------|
| `SLOT_DURATION` | 6000ms | L323 | Block time (`6 * 1000`) |
| `SLOTS_PER_EPOCH` | 300 | L112 | Slots before committee rotation |
| `MaxAuthorities` | 10,000 | L687 | Maximum validator set size |
| `MOTION_DURATION` | 5 days | L840 | Governance motion lifetime (`5 * DAYS`) |

## Usage

### Build

```bash
# Standard build (includes WASM)
cargo build -p midnight-node-runtime --release

# Build WASM only
cargo build -p midnight-node-runtime --release --target wasm32-unknown-unknown
```

### Run Benchmarks

```bash
cargo build -p midnight-node-runtime --release --features runtime-benchmarks
./target/release/midnight-node benchmark pallet \
    --chain dev \
    --pallet pallet_midnight \
    --extrinsic "*"
```

### Test Migrations

```bash
cargo build -p midnight-node-runtime --release --features try-runtime
./target/release/midnight-node try-runtime \
    --runtime ./target/release/wbuild/midnight-node-runtime/midnight_node_runtime.wasm \
    on-runtime-upgrade live --uri wss://node.example.com
```

## Runtime Version

The runtime version determines upgrade compatibility:

```rust
RuntimeVersion {
    spec_name: "midnight",
    spec_version: 000_018_001,  // Major.Minor.Patch encoded
    transaction_version: 2,
    // ...
}
```

- `spec_version` changes trigger runtime upgrades
- `transaction_version` changes indicate extrinsic format changes

## Integration

### Dependencies

This package integrates:
- `pallet-midnight` - Core ledger functionality
- `pallet-midnight-system` - System transactions
- `pallet-cnight-observation` - Cardano bridge
- `pallet-federated-authority` - Governance
- `runtime-common` - Shared runtime utilities

### Used By

- `midnight-node` (node) - Embeds runtime for block execution
- `midnight-node-metadata` - Generates subxt interfaces
- `midnight-node-toolkit` - Transaction generation

## Testing

```bash
# Unit tests
cargo test -p midnight-node-runtime

# With try-runtime checks
cargo test -p midnight-node-runtime --features try-runtime
```

## See Also

- [runtime-common](common/README.md) - Shared runtime components
- [pallet-midnight](../pallets/midnight/README.md) - Core ledger pallet
- [Chain Specifications](../docs/chain_specs.md) - Network configurations
- [Weights Documentation](../docs/weights.md) - [Benchmarking](https://docs.midnight.network/learn/glossary#benchmarking) guide

