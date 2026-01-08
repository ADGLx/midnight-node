# Scripts

Development, testing, and operational scripts for the Midnight blockchain.

## Overview

This directory contains development, testing, and operational scripts for the Midnight node. The scripts are organized into subdirectories for specific purposes: cNIGHT to DUST generation testing, partner chain development utilities, and end-to-end test automation. Individual Python and shell scripts at the root level provide common utilities for key generation, runtime analysis, and genesis configuration.

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
| [`analyse_runtime.sh`](analyse_runtime.sh) | Runtime analysis utilities |
| [`generate-genesis-seeds.py`](generate-genesis-seeds.py) | Generate genesis wallet seeds |
| [`generate-keys.py`](generate-keys.py) | Generate node keys (AURA, GRANDPA, etc.) |
| [`genesis_wallets_test.sh`](genesis_wallets_test.sh) | Genesis wallet testing |
| [`setup_sidechain.sh`](setup_sidechain.sh) | Sidechain setup automation |
| [`sync.sh`](sync.sh) | Network sync utilities |
| [`upgrade_test.sh`](upgrade_test.sh) | Runtime upgrade testing |

## Test Scripts

| Script | Description |
|--------|-------------|
| [`tests/genesis-wallets-devnet-e2e.sh`](tests/genesis-wallets-devnet-e2e.sh) | Genesis wallets on devnet |
| [`tests/genesis-wallets-undeployed-e2e.sh`](tests/genesis-wallets-undeployed-e2e.sh) | Genesis wallets on undeployed network |
| [`tests/hardfork-e2e.sh`](tests/hardfork-e2e.sh) | Hard fork scenario testing |
| [`tests/indexer-api-e2e.sh`](tests/indexer-api-e2e.sh) | Indexer API testing |
| [`tests/ledger-rollback-e2e.sh`](tests/ledger-rollback-e2e.sh) | Ledger rollback testing |
| [`tests/node-e2e.sh`](tests/node-e2e.sh) | Node integration testing |
| [`tests/startup-dev-e2e.sh`](tests/startup-dev-e2e.sh) | Dev network startup testing |
| [`tests/startup-qanet-e2e.sh`](tests/startup-qanet-e2e.sh) | QAnet startup testing |
| [`tests/toolkit-contracts-e2e.sh`](tests/toolkit-contracts-e2e.sh) | Toolkit contract operations |
| [`tests/toolkit-e2e.sh`](tests/toolkit-e2e.sh) | Toolkit integration testing |
| [`tests/toolkit-maintenance-e2e.sh`](tests/toolkit-maintenance-e2e.sh) | Toolkit maintenance operations |
| [`tests/toolkit-mint-e2e.sh`](tests/toolkit-mint-e2e.sh) | Toolkit minting operations |
| [`tests/toolkit-update-ledger-parameters-e2e.sh`](tests/toolkit-update-ledger-parameters-e2e.sh) | Ledger parameter updates |
| [`tests/toolkit-ut-e2e.sh`](tests/toolkit-ut-e2e.sh) | Toolkit unit test integration |

## See Also

- [local-environment/](../local-environment/README.md) - Docker-based network tools
- [tests/e2e/](../tests/e2e/README.md) - Rust E2E tests
- [CONTRIBUTING.md](../CONTRIBUTING.md) - Contribution guidelines

