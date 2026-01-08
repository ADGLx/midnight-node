[![Nightly Build Status](https://github.com/midnightntwrk/midnight-node/actions/workflows/nightly-build-check.yml/badge.svg?branch=main&event=schedule)](https://github.com/midnightntwrk/midnight-node/actions/workflows/nightly-build-check.yml?query=branch%3Amain)

# Midnight Node
<p align="center">
  <!-- Execution Stack (Indigo) -->
  <a href="node/README.md"><img src="https://img.shields.io/badge/Node-4f46e5?style=for-the-badge&logo=rust&logoColor=white" alt="Node"></a>
  <a href="runtime/README.md"><img src="https://img.shields.io/badge/Runtime-4f46e5?style=for-the-badge&logo=webassembly&logoColor=white" alt="Runtime"></a>
  <a href="pallets/README.md"><img src="https://img.shields.io/badge/Pallets-4f46e5?style=for-the-badge&logo=polkadot&logoColor=white" alt="Pallets"></a>
  <!-- Data Layer (Violet) -->
  <a href="primitives/README.md"><img src="https://img.shields.io/badge/Primitives-7c3aed?style=for-the-badge&logo=databricks&logoColor=white" alt="Primitives"></a>
  <a href="ledger/README.md"><img src="https://img.shields.io/badge/Ledger-7c3aed?style=for-the-badge&logo=bookstack&logoColor=white" alt="Ledger"></a>
  <!-- Developer Tools (Teal) -->
  <a href="util/README.md"><img src="https://img.shields.io/badge/Utils-0891b2?style=for-the-badge&logo=gnubash&logoColor=white" alt="Util"></a>
  <a href="scripts/README.md"><img src="https://img.shields.io/badge/Scripts-0891b2?style=for-the-badge&logo=gnubash&logoColor=white" alt="Scripts"></a>
  <!-- Infrastructure (Slate) -->
  <a href="res/README.md"><img src="https://img.shields.io/badge/Resources-475569?style=for-the-badge&logo=toml&logoColor=white" alt="Res"></a>
  <a href="local-environment/README.md"><img src="https://img.shields.io/badge/Local_Env-475569?style=for-the-badge&logo=docker&logoColor=white" alt="Local Environment"></a>
  <!-- Quality (Emerald) -->
  <a href="tests/README.md"><img src="https://img.shields.io/badge/Tests-059669?style=for-the-badge&logo=pytest&logoColor=white" alt="Tests"></a>
  <a href="docs/README.md"><img src="https://img.shields.io/badge/Docs-059669?style=for-the-badge&logo=readthedocs&logoColor=white" alt="Docs"></a>
</p>

Implementation of the Midnight blockchain node, providing consensus, transaction processing, and privacy-preserving smart contract execution. Built on [Substrate](https://github.com/paritytech/polkadot-sdk) and operating as a [Cardano Partner Chain](https://github.com/input-output-hk/partner-chains), the node enables participants to maintain both public blockchain state and private user state through zero-knowledge proofs.

Smart contracts written in [Compact](https://docs.midnight.network/learn/glossary#compact) execute within the [ZSwap](https://docs.midnight.network/learn/glossary#zswap) ledger, which provides cryptographic guarantees for transaction privacy while preserving on-chain verifiability. The node supports cross-chain token bridging between [cNIGHT](https://docs.midnight.network/learn/glossary#cnight) on Cardano and [DUST](https://docs.midnight.network/learn/glossary#dust) on Midnight, federated multi-body governance synchronized with the mainchain, and achieves finality through [AURA](https://docs.polkadot.com/polkadot-protocol/glossary#authority-round-aura) block production with [GRANDPA](https://docs.polkadot.com/polkadot-protocol/glossary#grandpa) and [BEEFY](https://github.com/paritytech/polkadot-sdk/blob/master/substrate/client/consensus/beefy/README.md) consensus mechanisms.

## Features

* **Privacy-Preserving Smart Contracts** - Execute contracts with zero-knowledge proofs while maintaining public blockchain state

* **Partner Chain Architecture** - Integrated with Cardano mainchain as a partner chain with cross-chain token bridging (cNIGHT to DUST)

* **Multi-Layer Governance** - Federated authority system requiring consensus from multiple governance bodies with automatic mainchain synchronization

* **High Performance** - 6-second block time with efficient finality mechanism and optimized transaction processing

* **Developer Tools** - Comprehensive CLI with chain specification generation, runtime benchmarking, and upgrade testing capabilities

## Architecture

```
┌────────────────────────────────────────────────────────────────────────────┐
│                        Cardano Partner Chain Stack                         │
└────────────────────────────────────────────────────────────────────────────┘
         │
         │ Observes mainchain state
         ▼
┌─────────────┐      ┌─────────────┐      ┌──────────────┐
│   Cardano   │ ───▶ │   db-sync   │ ───▶ │  PostgreSQL  │
│  Mainchain  │      │             │      │  (cexplorer) │
└─────────────┘      └─────────────┘      └──────────────┘
                                                   │
                                                   │ Queries Cardano data
                                                   │ (cNIGHT, governance)
                                                   ▼
     ┌────────────────────────────────────────────────────────────────────┐
◀──▶ │                         Midnight Node                              │ ◀──▶
P2P  ├────────────────────────────────────────────────────────────────────┤  P2P
Port │                                                                    │  Port
30333│  ┌──────────────────────────────────────────────────────────────┐  │  30333
     │  │                          Runtime                             │  │
     │  │                                                              │  │
     │  │  ┌────────────────────────────────────────────────────────┐  │  │
     │  │  │                       Pallets                          │  │  │
     │  │  │                                                        │  │  │
     │  │  │  ┌─────────────┐  ┌──────────────┐  ┌──────────────┐   │  │  │
     │  │  │  │  Midnight   │  │   Native     │  │  Federated   │   │  │  │
     │  │  │  │   System    │  │    Token     │  │  Authority   │   │  │  │
     │  │  │  │             │  │ Observation  │  │              │   │  │  │
     │  │  │  └─────────────┘  └──────────────┘  └──────────────┘   │  │  │
     │  │  │                                                        │  │  │
     │  │  │  ┌─────────────┐  ┌──────────────┐  ┌──────────────┐   │  │  │
     │  │  │  │   Version   │  │   Midnight   │  │  Federated   │   │  │  │
     │  │  │  │             │  │              │  │  Authority   │   │  │  │
     │  │  │  │             │  │              │  │ Observation  │   │  │  │
     │  │  │  └─────────────┘  └──────────────┘  └──────────────┘   │  │  │
     │  │  └────────────────────────────────────────────────────────┘  │  │
     │  └──────────────────────────────────────────────────────────────┘  │
     │                                                                    │
     │  ┌──────────────────────────────────────────────────────────────┐  │
     │  │                      Node Services                           │  │
     │  │                                                              │  │
     │  │    ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌──────────┐    │  │
     │  │    │   RPC    │  │Consensus │  │ Network  │  │ Keystore │    │  │
     │  │    │  Server  │  │   AURA   │  │   P2P    │  │          │    │  │
     │  │    │          │  │ GRANDPA  │  │          │  │          │    │  │
     │  │    └──────────┘  └──────────┘  └──────────┘  └──────────┘    │  │
     │  └──────────────────────────────────────────────────────────────┘  │
     └────────────────────────────────────────────────────────────────────┘
                                    │
                                    │ WebSocket RPC
                                    │ Port: 9944
                                    ▼
                         ┌──────────────────────┐
                         │   External Clients   │
                         │  (Wallets, Indexers, │
                         │     Applications)    │
                         └──────────────────────┘

     Other Midnight Nodes ◀────P2P Network (Port 30333)────▶ Other Midnight Nodes
```

> **Security Note:** Database connections to PostgreSQL require SSL/TLS by default. Set `ALLOW_NON_SSL=true` only for local development environments without SSL certificates.

## Quick Start

If you just want to run midnight-node, the easiest option is to use the Docker setup:

```shell
git clone https://github.com/midnightntwrk/midnight-node-docker
cd midnight-node-docker
docker compose up
```

## Prerequisites

- rustup installed
- For any docker steps: [Docker](https://docs.docker.com/get-docker/)
  and [Docker Compose](https://docs.docker.com/compose/install/) (or podman).
- [Earthly](https://earthly.dev/get-earthly) - containerized build system
- [Direnv](https://direnv.net/docs/installation.html) - manages environment variables

## Contributing

See [CONTRIBUTING.md](./CONTRIBUTING.md) for guidelines on contributing to this project.

## Documentation

| Topic | Description |
|-------|-------------|
| [Development Workflow](docs/development-workflow.md) | Environment setup, Cargo vs Earthly, debugging tips |
| [Local Environment](local-environment/README.md) | Docker-based networks, manual node startup |
| [Chain Specifications](res/README.md) | Genesis rebuilding, network configurations |
| [Toolkit Usage](util/toolkit/README.md) | Transaction generation, wallet management |
| [Scripts Reference](scripts/README.md) | Key generation, utility scripts |
| [Fork Testing](docs/fork-testing.md) | Hard fork testing procedures |

For a quick Earthly target reference, run `earthly doc` to list all available targets.

> [!NOTE]
> **Open Sourcing Progress:** While this repository is open source, it depends on repositories still being released. It's not possible to compile midnight-node independently yet, but PRs will compile via CI. We're actively working to open-source dependencies in the coming months.
