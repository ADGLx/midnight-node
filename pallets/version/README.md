# pallet-version

Pallet that records runtime spec version in block digests for monitoring and upgrade tracking.

## Overview

This pallet deposits the runtime's `spec_version` into each block's digest as a consensus log item. This enables:

- External monitoring tools to track runtime versions across blocks
- Detection of runtime upgrades by watching for version changes
- Light clients to verify runtime version without full block execution

## API Specification

### Constants

| Name | Value | Description |
|------|-------|-------------|
| `VERSION_ID` | `*b"MNSV"` | Consensus engine ID for version logs |

### Config Trait

| Associated Type | Description |
|-----------------|-------------|
| `WeightInfo` | Weight information for hooks |
| `RuntimeVersion` | `Get<RuntimeVersion>` - Runtime version provider |

### Hooks

| Hook | Weight | Description |
|------|--------|-------------|
| `on_initialize` | ~1 read | Deposits version to block digest |

### Public Functions

| Function | Signature | Description |
|----------|-----------|-------------|
| `decode_version` | `fn(&DigestItem) -> Option<u32>` | Extract version from digest item |

### Cargo Features

| Feature | Default | Description |
|---------|---------|-------------|
| `std` | Yes | Standard library support |
| `try-runtime` | No | Try-runtime support (stub) |

## Usage

### Runtime Configuration

```rust
impl pallet_version::Config for Runtime {
    type WeightInfo = pallet_version::VersionWeight<Runtime>;
    type RuntimeVersion = Version;  // parameter_types! constant
}
```

### Reading Version from Blocks

```rust
use pallet_version::{VERSION_ID, Pallet as NodeVersion};

// From a block digest
for log in block.header.digest.logs {
    if let Some(version) = NodeVersion::<Runtime>::decode_version(&log) {
        println!("Block produced with runtime version: {}", version);
    }
}
```

### Digest Format

The version is encoded as:
```
DigestItem::Consensus(VERSION_ID, spec_version.encode())
```

Where `VERSION_ID = b"MNSV"` (Midnight Node Spec Version).

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

- [runtime](../../runtime/README.md) - Runtime version configuration

