# Midnight Node Runtime

The [WASM](https://docs.polkadot.com/polkadot-protocol/glossary/#webassembly-wasm) runtime that defines the Midnight blockchain's state transition function, consensus rules, and on-chain logic.

## Overview

This crate compiles to [WebAssembly (WASM)](https://docs.polkadot.com/polkadot-protocol/glossary/#webassembly-wasm) and executes within the Substrate executor to process blocks and transactions. It composes [FRAME](https://docs.polkadot.com/polkadot-protocol/glossary/#frame-framework-for-runtime-aggregation-of-modularized-entities) pallets into a complete blockchain runtime, defining:

- **[Pallet](https://docs.polkadot.com/polkadot-protocol/glossary/#pallet) composition** - Which pallets are included and how they're configured
- **Consensus parameters** - [AURA](https://docs.polkadot.com/polkadot-protocol/glossary/#authority-round-aura) (6-second blocks), [GRANDPA](https://docs.polkadot.com/polkadot-protocol/glossary/#grandpa) finality, [BEEFY](https://docs.midnight.network/learn/glossary#beefy-bridge-efficiency-enabling-finality-yielder) bridge support
- **Type definitions** - Account IDs, balances, block numbers, signatures
- **[Runtime](https://docs.polkadot.com/polkadot-protocol/glossary/#runtime) APIs** - Interfaces for RPC queries and off-chain interactions
- **Storage migrations** - Upgrade logic for runtime changes

The runtime is the "business logic" of the chain—validators execute it identically to reach consensus on state transitions.

## API Specification

### Core Types

- [**`BlockNumber`**](https://github.com/midnightntwrk/midnight-node/blob/main/runtime/src/lib.rs#L139) - Index to a block in the chain (`u32`)
- [**`AccountId`**](https://github.com/midnightntwrk/midnight-node/blob/main/runtime/src/lib.rs#L146) - Account identifier derived from public key
- [**`Balance`**](https://github.com/midnightntwrk/midnight-node/blob/main/runtime/src/lib.rs#L149) - Account balance type (`u128`)
- [**`Nonce`**](https://github.com/midnightntwrk/midnight-node/blob/main/runtime/src/lib.rs#L152) - Transaction index for replay protection (`u32`)
- [**`Hash`**](https://github.com/midnightntwrk/midnight-node/blob/main/runtime/src/lib.rs#L155) - 256-bit hash for blocks and tries (`sp_core::H256`)
- [**`Signature`**](https://github.com/midnightntwrk/midnight-node/blob/main/runtime/src/lib.rs#L142) - Transaction signature (Sr25519/Ed25519/ECDSA)

### Block Structure

- [**`Block`**](https://github.com/midnightntwrk/midnight-node/blob/main/runtime/src/lib.rs#L1017) - Standard Substrate block containing header and [extrinsics](https://docs.polkadot.com/polkadot-protocol/glossary/#extrinsic)
- [**`Header`**](https://github.com/midnightntwrk/midnight-node/blob/main/runtime/src/lib.rs#L1015) - Block header with number, parent hash, state root, and Blake2-256 hashing
- [**`UncheckedExtrinsic`**](https://github.com/midnightntwrk/midnight-node/blob/main/runtime/src/lib.rs#L1031) - Transaction with sender address, call data, signature, and extensions
- [**`SignedExtra`**](https://github.com/midnightntwrk/midnight-node/blob/main/runtime/src/lib.rs#L1019) - Transaction extensions (nonce, mortality period, weight checks)

### Runtime APIs

- [**`MidnightRuntimeApi`**](https://github.com/midnightntwrk/midnight-node/blob/main/runtime/src/lib.rs#L1098) - Ledger state queries (contract state, transaction decoding, network ID)
- [**`SessionValidatorManagementApi`**](https://github.com/midnightntwrk/midnight-node/blob/main/runtime/src/lib.rs#L1480) - Committee queries (current/next validators)
- [**`CNightObservationApi`**](https://github.com/midnightntwrk/midnight-node/blob/main/runtime/src/lib.rs#L1515) - Cardano bridge configuration
- [**`FederatedAuthorityObservationApi`**](https://github.com/midnightntwrk/midnight-node/blob/main/runtime/src/lib.rs#L1561) - Governance address queries
- [**`GovernedMapIDPApi`**](https://github.com/midnightntwrk/midnight-node/blob/main/runtime/src/lib.rs#L1546) - Key-value governance map state
- [**`AuraApi`**](https://github.com/midnightntwrk/midnight-node/blob/main/runtime/src/lib.rs#L1199) - Block production slot duration and authorities
- [**`GrandpaApi`**](https://github.com/midnightntwrk/midnight-node/blob/main/runtime/src/lib.rs#L1354) - Finality authorities and set ID
- [**`BeefyApi`**](https://github.com/midnightntwrk/midnight-node/blob/main/runtime/src/lib.rs#L1209) - Bridge validator set and proofs
- [**`MmrApi`**](https://github.com/midnightntwrk/midnight-node/blob/main/runtime/src/lib.rs#L1284) - Merkle Mountain Range root and proofs

## Architecture

### Pallet Composition

The runtime composes FRAME pallets into five functional layers. System pallets provide core Substrate functionality (accounts, timestamps, administrative controls). Consensus pallets implement AURA block production, GRANDPA finality, and BEEFY bridge proofs. Midnight pallets handle the ZSwap ledger, system transactions, and Cardano observations. Governance pallets enable on-chain voting through Council and Technical Committee collectives, coordinated by the Federated Authority mechanism. Partner Chain pallets integrate with the Cardano mainchain for validator management and cross-chain state.

- **System Pallets**: System, Timestamp, Sudo, Scheduler, TxPause, Preimage, Migrations
- **Consensus Pallets**: Aura, Grandpa, Beefy, Mmr, BeefyMmrLeaf
- **Midnight Pallets**: Midnight, MidnightSystem, CNightObservation, NodeVersion
- **Governance Pallets**: Council, CouncilMembership, TechnicalCommittee, TechnicalCommitteeMembership, FederatedAuthority, FederatedAuthorityObservation
- **Partner Chain Pallets**: Sidechain, Session, SessionCommitteeManagement, GovernedMap

**Sources**: [[1]](https://github.com/midnightntwrk/midnight-node/blob/main/runtime/src/lib.rs#L924-L1010) [[2]](https://github.com/midnightntwrk/midnight-node/blob/main/runtime/src/lib.rs#L360-L410)

### Pallet Index Map

| Index | [Pallet](https://docs.polkadot.com/polkadot-protocol/glossary/#pallet) | Purpose |
|-------|--------|---------|
| 0 | System | Core frame system |
| 1 | Timestamp | Block timestamps |
| 2 | Aura | Block production |
| 3 | Grandpa | Finality |
| 5 | Midnight | Ledger state and transactions |
| 6 | MidnightSystem | System-level ledger operations |
| 7 | Sudo | Administrative operations |
| 11 | NodeVersion | [Runtime](https://docs.polkadot.com/polkadot-protocol/glossary/#runtime) version tracking |
| 13 | CNightObservation | [cNIGHT](https://docs.midnight.network/learn/glossary#cnight) token bridge |
| 21-23 | Beefy/Mmr | Bridge support |
| 40-45 | Governance | [Council](https://docs.midnight.network/learn/glossary#council), TC, [Federated Authority](https://docs.midnight.network/learn/glossary#federated-authority) |

### Consensus Configuration

- [**`SLOT_DURATION`**](https://github.com/midnightntwrk/midnight-node/blob/main/runtime/src/lib.rs#L333) - Block time in milliseconds
- [**`SLOTS_PER_EPOCH`**](https://github.com/midnightntwrk/midnight-node/blob/main/runtime/src/lib.rs#L116) - Slots before committee rotation
- [**`MaxAuthorities`**](https://github.com/midnightntwrk/midnight-node/blob/main/runtime/src/lib.rs#L577) - Maximum validator set size
- [**`MOTION_DURATION`**](https://github.com/midnightntwrk/midnight-node/blob/main/runtime/src/lib.rs#L728) - Governance motion lifetime

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

The runtime version determines upgrade compatibility. `spec_version` changes trigger runtime upgrades, while `transaction_version` changes indicate extrinsic format changes.

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
- [Weights Documentation](../docs/weights.md) - Benchmarking guide

