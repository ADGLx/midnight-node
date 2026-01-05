# Scripts

Development, testing, and operational scripts for the Midnight blockchain.

## Overview

```
+-----------------------------------------------------------------------+
|                             Scripts                                    |
+-----------------------------------------------------------------------+
| cnight-generates-dust/ | cNIGHT → DUST generation test scripts        |
| partnerchains-dev/     | Partner chain development utilities          |
| tests/                 | E2E test shell scripts                       |
| *.py, *.sh             | Utility scripts                               |
+-----------------------------------------------------------------------+
```

## Directories

### [cnight-generates-dust/](cnight-generates-dust/README.md)
Test scripts for cNIGHT → DUST generation scenarios including wallet registration and token movement.

### [partnerchains-dev/](partnerchains-dev/Readme.md)
Partner chain development utilities for key generation, UTXO creation, and network configuration.

### [tests/](tests/)
End-to-end test shell scripts for various node and toolkit scenarios.

## Utility Scripts

| Script | Description |
|--------|-------------|
| `analyse_runtime.sh` | Runtime analysis utilities |
| `generate-genesis-seeds.py` | Generate genesis wallet seeds |
| `generate-keys.py` | Generate node keys (AURA, GRANDPA, etc.) |
| `genesis_wallets_test.sh` | Genesis wallet testing |
| `setup_sidechain.sh` | Sidechain setup automation |
| `sync.sh` | Network sync utilities |
| `upgrade_test.sh` | Runtime upgrade testing |

## Test Scripts

| Script | Description |
|--------|-------------|
| `tests/genesis-wallets-devnet-e2e.sh` | Genesis wallets on devnet |
| `tests/hardfork-e2e.sh` | Hard fork scenario testing |
| `tests/indexer-api-e2e.sh` | Indexer API testing |
| `tests/node-e2e.sh` | Node integration testing |
| `tests/toolkit-e2e.sh` | Toolkit integration testing |

## See Also

- [local-environment/](../local-environment/README.md) - Docker-based network tools
- [tests/e2e/](../tests/e2e/README.md) - Rust E2E tests
- [CONTRIBUTING.md](../CONTRIBUTING.md) - Contribution guidelines

