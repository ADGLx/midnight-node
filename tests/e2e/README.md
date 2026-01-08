# tests/e2e

End-to-end integration tests for the Midnight node.

## Overview

This crate contains integration tests that exercise the full Midnight node stack, including:

- Transaction processing
- Consensus operations
- RPC endpoints
- Ledger state transitions

These tests are **not** run by default with `cargo test` due to their execution time and resource requirements.

## Running Tests

### CI Environment

```bash
cargo test --test e2e_tests
```

### Local Development

```bash
# Using the feature flag
cargo test --test e2e_tests --no-default-features --features local

# Using the cargo alias
cargo test-e2e-local
```

### Prerequisites

For local testing:

1. Build the node:
   ```bash
   cargo build --release -p midnight-node
   ```

2. Ensure no other node is running on default ports

## Test Structure

```
tests/e2e/
+-- Cargo.toml
+-- src/
|   +-- lib.rs          # Test utilities and helpers
+-- tests/
    +-- e2e_tests.rs    # Main test file
```


## Writing Tests

### Test Template

```rust
#[tokio::test]
async fn test_feature() {
    // Setup
    let node = TestNode::start().await;
    
    // Execute
    let result = node.submit_transaction(&tx).await;
    
    // Assert
    assert!(result.is_ok());
    
    // Cleanup (automatic on drop)
}
```

### Available Helpers

- **`TestNode`** - Spawns and manages test node
- **`TestClient`** - RPC client for node interaction
- **`TxBuilder`** - Transaction construction

## Integration

### Dependencies

- [`midnight-node-runtime`](https://github.com/midnightntwrk/midnight-node/blob/main/runtime/src/lib.rs) - Runtime under test
- [`midnight-node-ledger-helpers`](https://github.com/midnightntwrk/midnight-node/blob/main/ledger/helpers/src/lib.rs) - Transaction building
- `subxt` - RPC client (external)

### Related Tests

- `pallet-midnight` unit tests
- `runtime` integration tests

## See Also

- [node](../../node/README.md) - Node being tested
- [runtime](../../runtime/README.md) - [Runtime](https://docs.polkadot.com/polkadot-protocol/glossary/#runtime) being tested
- [ledger/helpers](../../ledger/helpers/README.md) - Test utilities
