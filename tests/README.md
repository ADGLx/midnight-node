# Tests

End-to-end and integration test suites.

## Overview

This directory contains integration and end-to-end tests for the Midnight node. The e2e package exercises the full node stack including consensus, transaction processing, and RPC interfaces. The redemption-skeleton package provides test fixtures specifically for validating Glacier Drop redemption contract behavior.

## Packages

### [e2e/](e2e/README.md)
**e2e** - End-to-end integration tests exercising the full node stack. Run separately from unit tests due to execution time and resource requirements.

### [redemption-skeleton/](redemption-skeleton/README.md)
Test fixture for Glacier Drop redemption contract validation.

## Package Index

| Package | Path | Description |
|---------|------|-------------|
| `e2e` | `e2e/` | E2E tests |
| `redemption-skeleton` | `redemption-skeleton/` | Redemption test fixtures |

## Running Tests

### Unit Tests
```bash
cargo test
```

### E2E Tests
```bash
cargo test --test e2e_tests
```

### Local E2E Tests
```bash
cargo test --test e2e_tests --no-default-features --features local
```

## See Also

- [node/](../node/README.md) - Node being tested
- [runtime/](../runtime/README.md) - Runtime being tested
- [CONTRIBUTING.md](../CONTRIBUTING.md) - Contribution guidelines

