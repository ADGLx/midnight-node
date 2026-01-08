# midnight-node-metadata

Generated subxt interfaces and metadata for Midnight protocol versions.

## Overview

This crate contains SCALE-encoded runtime metadata used by:
- `subxt` for type-safe extrinsic construction
- `util/toolkit` for transaction building
- `util/toolkit-js` for JavaScript bindings

## API Specification

### Modules

- **`midnight_metadata_latest`** - Latest protocol version interfaces
- **`midnight_metadata_X_Y_Z`** - Specific version (e.g., `midnight_metadata_0_17_0`)

### Static Files

Located in `static/`:

- **`*.scale`** - SCALE-encoded metadata for each version

## Usage

```rust
use midnight_node_metadata::midnight_metadata_latest;

// Access metadata types
let api = midnight_metadata_latest::RuntimeApi::new();
```

## Regenerating Metadata

To regenerate metadata after runtime changes:

```bash
# Start a node with the new runtime
./target/release/midnight-node --dev

# Generate metadata (from another terminal)
subxt metadata -f bytes > metadata/static/midnight_metadata_X_Y_Z.scale
```

## Integration

### Dependencies

- `subxt` - Type generation from metadata

### Used By

- [`util/toolkit`](https://github.com/midnightntwrk/midnight-node/blob/main/util/toolkit/src/lib.rs) - Transaction building CLI
- [`util/toolkit-js`](https://github.com/midnightntwrk/midnight-node/blob/main/util/toolkit-js/src/index.ts) - JavaScript/TypeScript bindings
- [`util/upgrader`](https://github.com/midnightntwrk/midnight-node/blob/main/util/upgrader/src/main.rs) - [Runtime](https://docs.polkadot.com/polkadot-protocol/glossary/#runtime) upgrade tool

## See Also

- [util/toolkit](../util/toolkit/README.md) - Primary consumer
- [res](../res/README.md) - Chain configuration resources
