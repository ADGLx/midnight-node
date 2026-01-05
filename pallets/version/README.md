# pallet-version

[Pallet](https://docs.midnight.network/learn/glossary#pallet) that records runtime spec version in block digests for monitoring and upgrade tracking.

## Overview

This pallet deposits the runtime's `spec_version` into each block's digest as a consensus log item. This enables external monitoring tools to track runtime versions across blocks, detection of runtime upgrades by watching for version changes, and light clients to verify runtime version without full block execution.

The version is recorded during `on_initialize` using the consensus engine ID `*b"MNSV"` (Midnight Node Spec Version). Block explorers and indexers can decode this log to build a historical timeline of runtime upgrades. The pallet has minimal weight impact since it only performs a single digest write per block with no storage reads beyond the runtime version constant.

## API Specification

### Constants

- [**`VERSION_ID`**](https://github.com/midnightntwrk/midnight-node/blob/main/pallets/version/src/lib.rs#L30) - Consensus engine ID for version logs (`*b"MNSV"`)

### Config Trait

- [**`WeightInfo`**](https://github.com/midnightntwrk/midnight-node/blob/main/pallets/version/src/lib.rs#L41) - Weight information for hooks
- [**`RuntimeVersion`**](https://github.com/midnightntwrk/midnight-node/blob/main/pallets/version/src/lib.rs#L43) - Provider that returns the current runtime version

### Hooks

- [**`on_initialize`**](https://github.com/midnightntwrk/midnight-node/blob/main/pallets/version/src/lib.rs#L51) - Deposits version to block digest

### Public Functions

- [**`decode_version`**](https://github.com/midnightntwrk/midnight-node/blob/main/pallets/version/src/lib.rs#L60) - Extract version from digest item

## Digest Format

The version is encoded as `DigestItem::Consensus(VERSION_ID, spec_version.encode())` where `VERSION_ID = b"MNSV"` (Midnight Node Spec Version).

## Integration

### Dependencies

- `sp-version` - RuntimeVersion type
- `frame-support` / `frame-system` - FRAME primitives

### Used By

- `runtime` - Block production
- External indexers/monitors - Version tracking

## Testing

```bash
cargo test -p pallet-version
```

## See Also

- [runtime](../../runtime/README.md) - [Runtime](https://docs.midnight.network/learn/glossary#runtime) version configuration

