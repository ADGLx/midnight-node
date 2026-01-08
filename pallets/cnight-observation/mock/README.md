# pallet-cnight-observation-mock

Mock runtime for testing pallet-cnight-observation.

## Overview

This crate provides a minimal mock runtime that configures `pallet-cnight-observation` for unit and integration testing. It sets up all required pallets (System, Timestamp, Midnight, MidnightSystem) with test-friendly configurations.

The mock runtime uses simplified types (u64 for AccountId, block numbers) and minimal genesis state to reduce test complexity. It exports `new_test_ext()` for creating fresh test externalities and provides access to all configured pallet instances. Test developers can simulate Cardano observations, verify CMST generation, and check event emissions without running a full node.

## Usage

### In Tests

```rust
use pallet_cnight_observation_mock::{new_test_ext, Test, CNightObservation};

#[test]
fn test_registration() {
    new_test_ext().execute_with(|| {
        // Test code using the mock runtime
        // CNightObservation is the configured pallet instance
    });
}
```

### Provided Components

| Component | Description |
|-----------|-------------|
| `Test` | Mock runtime type |
| `new_test_ext()` | Creates test externalities with genesis state |
| `CNightObservation` | Pallet instance alias |

## Configuration

The mock runtime includes:

- `frame_system` - Core system pallet
- `pallet_timestamp` - Timestamp for block context
- `pallet_midnight` - Ledger state management
- `pallet_midnight_system` - System transaction execution
- `pallet_cnight_observation` - The pallet under test

Genesis state is initialized from `midnight-node-res` test fixtures.

## Integration

### Dependencies

- `pallet-cnight-observation` - Pallet under test
- `midnight-node-res` - Test genesis data
- `pallet-midnight` / `pallet-midnight-system` - Required pallets

### Used By

- [`pallet-cnight-observation`](https://github.com/midnightntwrk/midnight-node/blob/main/pallets/cnight-observation/src/lib.rs) tests
- Integration tests requiring Cardano observation simulation

## See Also

- [pallet-cnight-observation](../README.md) - Main pallet

