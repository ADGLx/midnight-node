# toolkit-js

JavaScript/TypeScript CLI for executing [Compact](../../GLOSSARY.md#compact) compiled contracts.

## Overview

This toolkit provides commands for deploying and interacting with [Compact](../../GLOSSARY.md#compact) smart contracts. It requires a TypeScript configuration file that binds compiled contract output to its assets and provides [witness](../../GLOSSARY.md#witness) implementations.

## Installation

```bash
npm install -g midnight-node-toolkit-js
```

Or run locally:
```bash
npm start -- <command> [options]
```

## API Specification

### Commands

| Command | Description |
|---------|-------------|
| `deploy` | Deploy a new contract instance |
| `circuit` | Invoke a contract circuit |
| `maintain contract` | Update contract maintenance authority |
| `maintain circuit` | Update circuit verifier key |

### Global Options

| Option | Env Variable | Description |
|--------|--------------|-------------|
| `-c, --config <file>` | - | Contract config file (default: `contract.config.ts`) |
| `-o, --output <file>` | - | Output file for Intent data (default: `output.bin`) |
| `-p, --coin-public <key>` | `KEYS_COIN_PUBLIC` | [ZSwap](../../GLOSSARY.md#zswap) coin public key (hex/Bech32m) |

### Deploy Options

| Option | Env Variable | Description |
|--------|--------------|-------------|
| `-s, --signing <key>` | `KEYS_SIGNING` | BIP-340 signing key for CMA |

### Circuit Options

| Option | Description |
|--------|-------------|
| `-i, --input <file>` | Serialized onchain state file |
| `<address>` | Contract address |
| `<circuit_id>` | Circuit name to invoke |
| `<arg>...` | Arguments forwarded to circuit |

### Maintain Options

| Option | Description |
|--------|-------------|
| `-i, --input <file>` | Current contract state file |
| `-s, --signing <key>` | Signing key for maintenance |
| `<address>` | Contract address |

## Usage

### Contract Configuration File

Create `contract.config.ts`:

```typescript
import { CompiledContract, ContractExecutable } from '@midnight-ntwrk/compact-js/effect';
import { Contract as CounterContract } from './managed/counter/contract/index.cjs';

type PrivateState = { count: number };

const witnesses = {
  private_increment: ({ privateState }) => [
    { count: privateState.count + 1 }, 
    []
  ]
};

const createInitialPrivateState = () => ({ count: 0 });

export default {
  contractExecutable: CompiledContract.make<CounterContract>('CounterContract', CounterContract).pipe(
    CompiledContract.withWitnesses(witnesses),
    CompiledContract.withCompiledFileAssets('./managed/counter'),
    ContractExecutable.make
  ),
  createInitialPrivateState,
  config: {
    keys: {
      coinPublic: '<hex_key>',
    },
    network: 'undeployed'
  }
};
```

### Deploying a Contract

```bash
midnight-node-toolkit-js deploy -s <signing_key> <constructor_args>...
```

### Invoking a Circuit

```bash
midnight-node-toolkit-js circuit --input state.bin <address> <circuit_id> <args>...
```

### Contract Maintenance

Update signing authority:
```bash
midnight-node-toolkit-js maintain contract --input state.bin -s <current_key> <address> <new_signing_key>
```

Update circuit verifier:
```bash
midnight-node-toolkit-js maintain circuit --input state.bin -s <signing_key> <address> <circuit_id> <verifier_key_path>
```

## Architecture

```
+------------------+     +------------------+     +------------------+
| contract.config  | --> | toolkit-js CLI   | --> | Intent Output    |
| .ts              |     |                  |     | (output.bin)     |
+------------------+     +------------------+     +------------------+
        |                        |
        v                        v
+------------------+     +------------------+
| Compiled         |     | @midnight-ntwrk/ |
| Contract Assets  |     | compact-js       |
| (.cjs, .zkir)    |     |                  |
+------------------+     +------------------+
```

### Configuration Resolution

```
contract.config.ts → Environment Variables → CLI Options
      (lowest)              ↓                (highest)
                    Priority increases →
```

## Integration

### Dependencies

- `@midnight-ntwrk/compact-js` - [Compact](../../GLOSSARY.md#compact) runtime
- `compactc` output - Compiled contract files

### Used By

- Contract developers for deployment
- Testing and CI/CD pipelines

## See Also

- [util/toolkit](../toolkit/README.md) - Rust CLI toolkit
- [Compact Language](../../GLOSSARY.md#compact) - Contract language
- [Witness](../../GLOSSARY.md#witness) - Private inputs definition
