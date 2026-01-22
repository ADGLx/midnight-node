# Midnight Network Tools

A flexible set of tools for launching **well-known networks, custom networks, and dynamic local environments**, as well as **performing state changes** against those networks (image upgrades now, runtime upgrades and hard forks coming soon).

This project provides a unified way to spin up Midnight resources for development, testing, and experimentation.

---

## Features

- Launch dockerized **well-known Midnight networks** (e.g. `qanet`, `devnet`, `testnet-02`, etc.)
- Perform **state-changing operations** such as image upgrades (runtime upgrades and hard forks planned).
- Launch a fully **dynamic local environment** with sped-up Cardano resources for quick testing of Partner Chains/Cardano capabilities.

---

## Usage

All functionality is available via npm/yarn scripts defined in `package.json`.

### Launching Networks

You can run different Midnight networks locally with:

```bash
npm run run:qanet
npm run run:devnet
npm run run:testnet-02
npm run run:node-dev-01
```

### Upgrading Networks

You can also launch a network and immediately apply image upgrades:

```bash
npm run image-upgrade:qanet
npm run image-upgrade:devnet
npm run image-upgrade:testnet-02
npm run image-upgrade:node-dev-01
```

### Stopping Networks

To stop any running network:

```bash
npm run stop:qanet
npm run stop:devnet
npm run stop:testnet-02
npm run stop:node-dev-01
```

### Fork Testing

See [fork-testing.md](../docs/fork-testing.md)

## Local Environment

In addition to well-known networks, you can launch a dynamic local environment that connects multiple components together.

### Prerequisites

Before starting the local environment, ensure you have:

1. **Docker and Docker Compose** installed and running

2. **midnight-reserve-contracts repository** cloned as a sibling directory to midnight-node:

```
parent-directory/
├── midnight-node/
└── midnight-reserve-contracts/
```

Clone it from: https://github.com/midnightntwrk/midnight-reserve-contracts

3. **Environment variables configured** by sourcing the `.envrc` file:

```bash
cd local-environment
source .envrc
```

This sets `MIDNIGHT_RESERVE_CONTRACTS_PATH` and other required variables. You should see output confirming the architecture, node version, and contracts path.

### Architecture

The local environment orchestrates multiple services that work together:

```
┌─────────────────────────────────────────────────────────────────────┐
│                         STARTUP SEQUENCE                            │
└─────────────────────────────────────────────────────────────────────┘

  1. Cardano Node
     │
     │ Generates one-shot UTxO hashes
     ▼
  2. Contract Compiler ──────────────────────────────────────────────┐
     │                                                                │
     │ Compiles Aiken governance contracts from                      │
     │ midnight-reserve-contracts using the UTxO hashes              │
     │                                                                │
     │ Outputs: CBOR files and policy IDs to /runtime-values         │
     ▼                                                                │
  3. Ogmios + DB-Sync                                                 │
     │                                                                │
     │ Connect to Cardano and sync chain data                        │
     ▼                                                                │
  4. Midnight Setup ◄─────────────────────────────────────────────────┘
     │
     │ Deploys compiled governance contracts to Cardano
     │ Generates chain-spec.json with contract addresses
     ▼
  5. Midnight Nodes (1, 2, 3)
     │
     │ Start block production using generated chain-spec
     ▼
  6. Proof Server + Supporting Services
```

**Key Services:**

| Service | Purpose |
|---------|---------|
| cardano-node | Private Cardano testnet for mainchain simulation |
| contract-compiler | Compiles Aiken governance contracts (one-shot, exits after completion) |
| ogmios | WebSocket API for Cardano node interaction |
| db-sync | Indexes Cardano chain data into PostgreSQL |
| midnight-setup | Deploys contracts and generates chain configuration |
| midnight-node-{1,2,3} | Midnight blockchain nodes |
| proof-server | Handles zero-knowledge proof generation |

> **Note:** Local development environments set `ALLOW_NON_SSL=true` to allow connections to PostgreSQL without SSL certificates. Production deployments require SSL.

### Starting the Environment

There are two ways to start the local environment: using Earthly or npm scripts.

#### Using Earthly

Earthly provides containerized, reproducible builds. Use this when you want consistent behavior across environments.

Start with the latest local node image:

```bash
earthly +start-local-env-latest
```

Or specify a released node image:

```bash
earthly +start-local-env --NODE-IMAGE=ghcr.io/midnight-ntwrk/midnight-node:0.12.0
```

#### Using npm Scripts

npm scripts are convenient for quick local development:

```bash
npm run run:local-env
```

When first run, all images are pulled from public repositories. This may take some time. Once started, Midnight nodes begin block production after approximately 2 main chain epochs.

### Using the Indexer

The indexer variant includes additional services for querying blockchain data:

```bash
npm run run:local-env-with-indexer
```

This adds:
- **chain-indexer** - Indexes Midnight blockchain data
- **wallet-indexer** - Indexes wallet-specific data
- **indexer-api** - REST API for querying indexed data

Use the indexer variant when developing applications that need to query historical blockchain state or transaction data.

### Stopping the Environment

When stopping, volumes must also be wiped (persistent state is not supported yet).

Using Earthly:

```bash
earthly +stop-local-env-latest
```

Or with a specific node image:

```bash
earthly +stop-local-env --NODE-IMAGE=ghcr.io/midnight-ntwrk/midnight-node:0.12.0
```

Using npm:

```bash
npm run stop:local-env
```

### Troubleshooting

**Contract compiler fails to start**
- Ensure `midnight-reserve-contracts` is cloned as a sibling directory
- Verify `.envrc` has been sourced (run `source .envrc`)
- Check that `MIDNIGHT_RESERVE_CONTRACTS_PATH` is set correctly

**Services fail to connect**
- Ensure Docker is running and has sufficient resources allocated
- Check for port conflicts (common ports: 30000, 32000, 9944)
- Try stopping and removing all containers: `docker compose down -v`

**Environment variables not set**
- Run `source .envrc` from the `local-environment` directory
- Verify output shows the architecture, node version, and contracts path

**First startup is slow**
- Initial image pulls can take several minutes
- Subsequent startups will be faster as images are cached
