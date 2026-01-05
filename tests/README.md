# Tests

End-to-end and integration test suites.

## Overview

```
+-----------------------------------------------------------------------+
|                              Tests                                     |
+-----------------------------------------------------------------------+
| e2e/                  | Full node stack integration tests             |
| redemption-skeleton/  | Glacier Drop redemption test fixtures         |
+-----------------------------------------------------------------------+
```

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

