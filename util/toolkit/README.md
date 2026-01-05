# midnight-toolkit

CLI tool for interacting with the Midnight blockchain.

## Overview

A comprehensive command-line toolkit supporting transaction generation, wallet management, contract deployment, and testing. The toolkit can operate in file-to-file, file-to-chain, chain-to-file, and chain-to-chain modes.

> **📚 See Usage Examples:** The best way to understand this CLI is by examining the end-to-end test scripts at [scripts/tests/toolkit-*.sh](https://github.com/midnightntwrk/midnight-node/tree/main/scripts/tests)

## Implementation Status

| Feature | Status |
|---------|--------|
| Send Shielded + Unshielded tokens | ✅ |
| Sync with local/remote networks | ✅ |
| [DUST](https://docs.midnight.network/learn/glossary#dust) fee calculation | ✅ |
| Execute compiled contracts | ✅ |
| Performance testing (100s of txs) | ✅ |
| [Runtime](https://docs.midnight.network/learn/glossary#runtime) fork support | ✅ |
| Wallet state and balance queries | ✅ |
| [Genesis](https://docs.midnight.network/learn/glossary#genesis) generation | ✅ |
| Contract maintenance (authority + verifier keys) | ✅ |
| [DUST](https://docs.midnight.network/learn/glossary#dust) registration | 🚧 |
| Contracts receiving tokens from user | 🚧 |
| Ledger fork support | ⏳ |
| Fallible/Composable contracts | ⏳ |

> **⚠️** Implementation status reflects development state as of documentation creation. Verify current status against codebase and release notes.

## API Specification

### Commands

- [**`version`**](https://github.com/midnightntwrk/midnight-node/blob/main/util/toolkit/src/commands/version.rs) - Show Node, Ledger, and Compactc versions
- [**`generate-txs`**](https://github.com/midnightntwrk/midnight-node/blob/main/util/toolkit/src/tx_generator/mod.rs) - Generate and send transactions
- [**`generate-genesis`**](https://github.com/midnightntwrk/midnight-node/blob/main/util/toolkit/src/commands/generate_genesis.rs) - Generate genesis block
- [**`generate-intent`**](https://github.com/midnightntwrk/midnight-node/blob/main/util/toolkit/src/commands/generate_intent.rs) - Generate contract intent via toolkit-js
- [**`send-intent`**](https://github.com/midnightntwrk/midnight-node/blob/main/util/toolkit/src/commands/send_intent.rs) - Build and send transaction from intent
- [**`show-transaction`**](https://github.com/midnightntwrk/midnight-node/blob/main/util/toolkit/src/commands/show_transaction.rs) - Deserialize and display transaction
- [**`show-wallet`**](https://github.com/midnightntwrk/midnight-node/blob/main/util/toolkit/src/commands/show_wallet.rs) - Display wallet state as JSON
- [**`show-address`**](https://github.com/midnightntwrk/midnight-node/blob/main/util/toolkit/src/commands/show_address.rs) - Get address for a seed
- [**`dust-balance`**](https://github.com/midnightntwrk/midnight-node/blob/main/util/toolkit/src/commands/dust_balance.rs) - Show DUST balance breakdown
- [**`contract-address`**](https://github.com/midnightntwrk/midnight-node/blob/main/util/toolkit/src/commands/contract_address.rs) - Extract contract address from deploy tx
- [**`contract-state`**](https://github.com/midnightntwrk/midnight-node/blob/main/util/toolkit/src/commands/contract_state.rs) - Get contract on-chain state
- [**`random-address`**](https://github.com/midnightntwrk/midnight-node/blob/main/util/toolkit/src/commands/random_address.rs) - Generate random address
- [**`get-tx-from-context`**](https://github.com/midnightntwrk/midnight-node/blob/main/util/toolkit/src/commands/get_tx_from_context.rs) - Extract transaction from context

### Transaction Generator Components

- **Source** - Where to read state (`--src-file` or `--src-url`)
- **Destination** - Where to send (`--dest-file` or `--dest-url`)
- **Prover** - Local or remote proof server
- **Builder** - Transaction build strategy

### Builder Subcommands

- [**`send`**](https://github.com/midnightntwrk/midnight-node/blob/main/util/toolkit/src/tx_generator/builder/builders/send.rs) - Pass-through from JSON file
- [**`single-tx`**](https://github.com/midnightntwrk/midnight-node/blob/main/util/toolkit/src/tx_generator/builder/builders/single_tx.rs) - Single tx to N destinations
- [**`migrate`**](https://github.com/midnightntwrk/midnight-node/blob/main/util/toolkit/src/tx_generator/builder/builders/migrate.rs) - Migrate txs between chains
- [**`batches`**](https://github.com/midnightntwrk/midnight-node/blob/main/util/toolkit/src/tx_generator/builder/builders/batches.rs) - Generate ZSwap & Unshielded UTXO batches
- [**`claim-mint`**](https://github.com/midnightntwrk/midnight-node/blob/main/util/toolkit/src/tx_generator/builder/builders/claim_mint.rs) - Build claim mint transactions
- [**`contract-simple deploy`**](https://github.com/midnightntwrk/midnight-node/blob/main/util/toolkit/src/tx_generator/builder/builders/contract_simple.rs) - Deploy built-in contract
- [**`contract-simple maintenance`**](https://github.com/midnightntwrk/midnight-node/blob/main/util/toolkit/src/tx_generator/builder/builders/contract_simple.rs) - Update contract authority/verifiers
- [**`contract-simple call`**](https://github.com/midnightntwrk/midnight-node/blob/main/util/toolkit/src/tx_generator/builder/builders/contract_simple.rs) - Call contract entrypoint
- [**`register-dust-address`**](https://github.com/midnightntwrk/midnight-node/blob/main/util/toolkit/src/tx_generator/builder/builders/register_dust.rs) - Register wallet for DUST generation

## Usage

### Check Version

```bash
midnight-node-toolkit version
```

### Generate Transactions

```bash
midnight-node-toolkit generate-txs <SRC_ARGS> <DEST_ARGS> <PROVER_ARG> <builder> <BUILDER_ARGS>
```

#### ZSwap & Unshielded Batches

```bash
# Chain to chain
midnight-node-toolkit generate-txs batches -n 1 -b 2

# File to file
midnight-node-toolkit generate-txs --dest-file txs.json batches -n 5 -b 1

# File to chain with rate control
midnight-node-toolkit generate-txs -r 2 --src-file txs.json --dest-url ws://127.0.0.1:9944 send
```

#### Single Transaction

```bash
midnight-node-toolkit generate-txs \
  single-tx \
  --shielded-amount 100 \
  --unshielded-amount 5 \
  --source-seed "0000...0001" \
  --destination-address mn_shield-addr_undeployed1...
```

#### Deploy Contract (Built-in)

```bash
midnight-node-toolkit generate-txs \
  contract-simple deploy \
  --rng-seed '0000...0037'
```

#### Call Contract (Built-in)

```bash
midnight-node-toolkit generate-txs \
  contract-simple call \
  --call-key store \
  --contract-address 3102ba67...
```

### Custom Contracts

Custom contracts require [toolkit-js](../toolkit-js/README.md). Set `TOOLKIT_JS_PATH` environment variable.

```bash
# Get coin-public-key
midnight-node-toolkit show-address \
    --network undeployed \
  --seed 0000...0001 \
  --coin-public

# Generate deploy intent
midnight-node-toolkit generate-intent deploy \
  -c toolkit-js/contract/contract.config.ts \
  --coin-public aa0d72bb... \
  --output-intent out/intent.bin

# Send intent as transaction
midnight-node-toolkit send-intent \
  --intent-file out/intent.bin \
  --compiled-contract-dir contract/counter/out
```

### Register DUST Address

```bash
midnight-node-toolkit generate-txs \
    --src-files "res/genesis/genesis_block_undeployed.mn" \
    --dest-file "register.mn" \
    --to-bytes \
    register-dust-address \
  --wallet-seed "0000...0000" \
  --funding-seed "0000...0001"
```

### Show Wallet State

```bash
midnight-node-toolkit show-wallet \
  --src-file res/genesis/genesis_block_undeployed.mn \
  --seed 0000...0001
```

### Dust Balance

```bash
midnight-node-toolkit dust-balance \
  --src-file res/genesis/genesis_block_undeployed.mn \
  --seed 0000...0001
```

## Architecture

```
+------------------+     +------------------+     +------------------+
| Source           |     | TxGenerator      |     | Destination      |
| - File (.mn)     | --> | - Builder        | --> | - File (.mn)     |
| - Chain (RPC)    |     | - Prover         |     | - Chain (RPC)    |
+------------------+     +------------------+     +------------------+
                                |
                                v
                         +------------------+
                         | Builders         |
                         | - batches        |
                         | - single-tx      |
                         | - contract-*     |
                         +------------------+
```

**Sources**: [[1]](https://github.com/midnightntwrk/midnight-node/blob/main/util/toolkit/src/tx_generator/mod.rs#L51) [[2]](https://github.com/midnightntwrk/midnight-node/blob/main/util/toolkit/src/tx_generator/builder/mod.rs#L16-L18)

### Data Flow Modes

| Mode | Source | Destination | Use Case |
|------|--------|-------------|----------|
| File → File | `.mn` file | `.mn` file | Transform/batch |
| File → Chain | `.mn` file | RPC | Replay transactions |
| Chain → File | RPC | `.mn` file | Export state |
| Chain → Chain | RPC | RPC | Live operations |

## Integration

### Dependencies

- `midnight-node-ledger-helpers` - Transaction building
- `mn-ledger` - Ledger types and proving
- `subxt` - Substrate RPC client
- `toolkit-js` (optional) - Custom contract support

### Used By

- CI/CD pipelines for testing
- Developers for local development
- Performance testing

## Development

### Adding a New Builder

1. Create struct in `util/toolkit/src/tx_generator/builder/builders` implementing `BuildTxs`
2. Add subcommand to `enum Builder`
3. Handle in `TxGenerator::builder()`

### Adding a New Contract

Create struct in `ledger/helpers/src/contract/contracts` implementing `Contract<D>`

## Docker

### Build Image

```bash
cd ../..
earthly +generator-image
```

### Run with Docker

```bash
# Access localhost node
docker run --network host midnight-node-toolkit:latest ...

# Write output to host
docker run --network host -v $(pwd):/out midnight-node-toolkit:latest \
  --dest-file /out/tx.json ...
```

## Testing

```bash
cargo test -p midnight-toolkit
```

## See Also

- [toolkit-js](../toolkit-js/README.md) - JavaScript CLI for custom contracts
- [scripts/tests](../../scripts/tests/) - End-to-end test examples
- [Glossary](https://docs.midnight.network/learn/glossary) - Term definitions
