# Compact Contract Deployment and Node Interaction

A detailed guide to deploying [Compact](../GLOSSARY.md#compact) smart contracts on the Midnight blockchain and understanding how they interact with the node.

## Overview

Midnight uses [Compact](../GLOSSARY.md#compact), a domain-specific language for defining privacy-preserving smart contracts [1]. The deployment process involves several stages: compilation, intent generation, proving, and submission to the node. This document covers the complete lifecycle from source code to on-chain contract state.

## Architecture Overview

```
+-------------------+     +-------------------+     +-------------------+
|   Compact Source  | --> |     compactc      | --> | Compiled Assets   |
|   (.compact)      |     |   (Compiler)      |     | (.cjs, .zkir,     |
|                   |     |                   |     |  .prover, .verifier)|
+-------------------+     +-------------------+     +-------------------+
                                                            |
                                                            v
+-------------------+     +-------------------+     +-------------------+
| Intent File       | <-- | toolkit-js        | <-- | contract.config.ts|
| (.bin)            |     | (Intent Gen)      |     | (Witnesses)       |
+-------------------+     +-------------------+     +-------------------+
         |
         v
+-------------------+     +-------------------+     +-------------------+
| Proven TX         | <-- | Proof Server      | <-- | midnight-toolkit  |
| (Signed)          |     | (Local/Remote)    |     | (Rust CLI)        |
+-------------------+     +-------------------+     +-------------------+
         |
         v
+-------------------+     +-------------------+     +-------------------+
| Node RPC          | --> | pallet-midnight   | --> | Ledger State      |
| (WebSocket)       |     | (send_mn_tx)      |     | (Updated)         |
+-------------------+     +-------------------+     +-------------------+
```

## Stage 1: Contract Compilation

### Compiling Compact Source

The `compactc` compiler transforms Compact source code into executable artifacts [2]:

```bash
compactc counter.compact ./output/counter
```

### Generated Artifacts

| Artifact | Description |
|----------|-------------|
| `contract/index.cjs` | JavaScript contract interface [3] |
| `keys/*.zkir` | Zero-knowledge circuit definitions |
| `keys/*.prover` | Prover keys for each circuit |
| `keys/*.verifier` | Verifier keys for on-chain verification |

### Version Compatibility

Contract artifacts are version-locked. Check compatibility [4]:

```bash
midnight-node-toolkit version
# Node: X.Y.Z
# Ledger: A.B.C
# Compactc: P.Q.R
```

## Stage 2: Intent Generation

An **Intent** is an intermediate representation capturing the contract action (deploy, call, maintain) along with witness data and private state transitions [5].

### Configuration File (contract.config.ts)

The configuration file binds compiled contract output to witness implementations [3]:

```typescript
import { CompiledContract, ContractExecutable } from '@midnight-ntwrk/compact-js/effect';
import { Contract as CounterContract } from './managed/counter/contract/index.cjs';

type PrivateState = { count: number };

// Witness implementations for private computations
const witnesses = {
  private_increment: ({ privateState }) => [
    { count: privateState.count + 1 }, 
    []
  ]
};

export default {
  contractExecutable: CompiledContract.make<CounterContract>('CounterContract', CounterContract).pipe(
    CompiledContract.withWitnesses(witnesses),
    CompiledContract.withCompiledFileAssets('./managed/counter'),
    ContractExecutable.make
  ),
  createInitialPrivateState: () => ({ count: 0 }),
  config: {
    keys: { coinPublic: '<hex_key>' },
    network: 'undeployed'
  }
};
```

### Generating Deploy Intent (toolkit-js)

```bash
midnight-node-toolkit-js deploy \
  -c contract.config.ts \
  --coin-public <public_key> \
  --output intent.bin \
  --output-ps private_state.json \
  --output-zswap zswap.json
```

### Generating Deploy Intent (Rust toolkit)

The Rust toolkit delegates to toolkit-js for intent generation [6]:

```bash
midnight-node-toolkit generate-intent deploy \
  -c ../toolkit-js/test/contract/contract.config.ts \
  --toolkit-js-path ../toolkit-js/ \
  --coin-public aa0d72bb77ea46f986a800c66d75c4e428a95bd7e1244f1ed059374e6266eb98 \
  --output-intent out/intent.bin \
  --output-private-state out/private_state.json \
  --output-zswap-state out/zswap.json \
  0  # Constructor argument
```

### Intent Structure

The intent encapsulates [7]:

| Component | Description |
|-----------|-------------|
| Contract Deploy Data | ZKIR circuits, verifier keys, initial state |
| Authority Committee | Maintenance authority public keys |
| Committee Threshold | Required signatures for maintenance |
| Time-to-Live (TTL) | Transaction validity window |

## Stage 3: Transaction Proving

Proving generates [SNARK](../GLOSSARY.md#snark-succinct-non-interactive-argument-of-knowledge) proofs for the transaction, ensuring privacy guarantees hold [8].

### Local Proving

```bash
midnight-node-toolkit send-intent \
  --intent-file intent.bin \
  --compiled-contract-dir ./contract/out
```

### Remote Proving

```bash
midnight-node-toolkit send-intent \
  --intent-file intent.bin \
  --compiled-contract-dir ./contract/out \
  --proof-server http://proof-server:8080
```

### Proof Generation Flow

The proof server (local or remote) generates Halo2 SNARK proofs [9]:

```
Intent → Unproven Transaction → Proof Server → Proven Transaction
                                     |
                                     v
                              +-------------+
                              | Halo2 SNARK |
                              | Proving     |
                              +-------------+
                                     |
                                     v
                              +-------------+
                              | Proof       |
                              | Attached    |
                              +-------------+
```

### Transaction Components

A finalized transaction contains [10]:

| Component | Description |
|-----------|-------------|
| `network_id` | Network identifier (e.g., "undeployed", "testnet") |
| `intents` | Map of segment ID → Intent (deploy/call actions) |
| `guaranteed_offer` | Shielded coin transfers (always applied) |
| `fallible_offer` | Conditional transfers (may fail) |
| `proofs` | SNARK proofs for each circuit invocation |
| `ttl` | Timestamp after which TX is invalid |

## Stage 4: Node Submission

### WebSocket RPC Connection

Transactions are submitted via WebSocket to the node's JSON-RPC interface [11]:

```
Default endpoint: ws://127.0.0.1:9944
```

### Submission Flow

The toolkit uses `subxt` to create and submit unsigned extrinsics [12]:

```rust
// From util/toolkit/src/sender.rs
let mn_tx = mn_meta::tx().midnight().send_mn_transaction(tx_serialize);
let unsigned_extrinsic = api.tx().create_unsigned(&mn_tx)?;
let tx_progress = unsigned_extrinsic.submit_and_watch().await?;
```

### Transaction Lifecycle

```
SENDING → SENT → BEST_BLOCK → FINALIZED
                     |
                     v
              (Event emitted)
```

## Stage 5: Node Processing

### pallet-midnight Extrinsic

The `send_mn_transaction` extrinsic processes Midnight transactions [13]:

```rust
#[pallet::call_index(0)]
pub fn send_mn_transaction(
    _origin: OriginFor<T>, 
    midnight_tx: Vec<u8>
) -> DispatchResult {
    // 1. Deserialize and validate transaction
    // 2. Apply to ledger state  
    // 3. Emit events for operations
    // 4. Update state root
}
```

### Validation (Pre-dispatch)

Before inclusion in a block, the transaction is validated against the current ledger state [13]:

```rust
fn validate_unsigned(call: &Call<T>, block_context: BlockContext) -> TransactionValidity {
    // Validate transaction against current ledger state
    // Check proofs, balances, TTL
    // Return transaction hash as "provides" tag
}
```

### Ledger State Update

The ledger bridge applies transactions via host functions [14]:

```
Current State Root → Apply Transaction → New State Root
                            |
                            v
                     +----------------+
                     | Update:        |
                     | - ZSwap tree   |
                     | - Contract     |
                     |   state        |
                     | - UTXOs        |
                     +----------------+
```

## API Endpoints

### Midnight-Specific RPC Methods

The pallet-midnight-rpc crate exposes custom JSON-RPC methods [15]:

| Method | Description | Parameters |
|--------|-------------|------------|
| `midnight_contractState` | Get contract state | `contract_address`, `at` (optional block) |
| `midnight_zswapStateRoot` | Get ZSwap Merkle root | `at` (optional block) |
| `midnight_ledgerVersion` | Get ledger version | `at` (optional block) |
| `midnight_apiVersions` | Get supported API versions | None |

### Usage Examples

#### Get Contract State

```bash
curl -X POST http://localhost:9933 \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "id": 1,
    "method": "midnight_contractState",
    "params": ["<contract_address_hex>", null]
  }'
```

#### Response

```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "result": "<serialized_contract_state_hex>"
}
```

### Standard Substrate RPC

| Method | Description |
|--------|-------------|
| `author_submitExtrinsic` | Submit unsigned extrinsic |
| `author_submitAndWatchExtrinsic` | Submit and subscribe to status |
| `chain_getBlockHash` | Get block hash by number |
| `state_getStorage` | Get raw storage value |

## Events

### Contract Events

Events emitted by pallet-midnight [13]:

| Event | Emitted When |
|-------|--------------|
| `ContractDeploy { tx_hash, contract_address }` | New contract deployed |
| `ContractCall { tx_hash, contract_address }` | Contract entrypoint invoked |
| `ContractMaintain { tx_hash, contract_address }` | Authority or verifier updated |

### Transaction Events

| Event | Emitted When |
|-------|--------------|
| `TxApplied { tx_hash }` | Transaction fully applied |
| `TxPartialSuccess { tx_hash }` | Guaranteed part applied, fallible failed |
| `UnshieldedTokens { spent, created }` | UTXO transfers |

### Subscribing to Events

```javascript
const api = await ApiPromise.create({ provider: wsProvider });
api.query.system.events((events) => {
  events.forEach((record) => {
    const { event } = record;
    if (event.section === 'midnight') {
      console.log(`${event.method}: ${event.data}`);
    }
  });
});
```

## Complete Deployment Example

### 1. Compile Contract

```bash
compactc counter.compact ./managed/counter
```

### 2. Generate Intent

```bash
midnight-node-toolkit generate-intent deploy \
  -c contract.config.ts \
  --toolkit-js-path ./toolkit-js/ \
  --coin-public $(midnight-node-toolkit show-address --network undeployed --seed <seed> --coin-public) \
  --output-intent out/deploy_intent.bin \
  --output-private-state out/ps.json \
  --output-zswap-state out/zswap.json \
  0
```

### 3. Build and Send Transaction

```bash
midnight-node-toolkit send-intent \
  --intent-file out/deploy_intent.bin \
  --compiled-contract-dir ./managed/counter
```

### 4. Get Contract Address

```bash
midnight-node-toolkit contract-address \
  --src-file deploy_tx.mn
# Output: 040dcc237a542543f1c0e0af4a8e937f74f357a238c9d2a9fcfcd644eb0f5c70
```

### 5. Query Contract State

```bash
midnight-node-toolkit contract-state \
  --src-url ws://127.0.0.1:9944 \
  --contract-address 040dcc237a542543f1c0e0af4a8e937f74f357a238c9d2a9fcfcd644eb0f5c70 \
  --dest-file contract_state.bin
```

## Contract Maintenance

After deployment, contract authority holders can update verifier keys or transfer authority [16]:

### Update Signing Authority

```bash
midnight-node-toolkit generate-intent maintain-contract \
  --contract-address <address> \
  --signing <current_key> \
  <new_signing_key>
```

### Update Circuit Verifier

```bash
midnight-node-toolkit generate-intent maintain-circuit \
  --contract-address <address> \
  --signing <key> \
  <circuit_id> \
  <new_verifier_key_path>
```

## Error Handling

### Common Errors

Errors defined in the pallet-midnight error enum [13]:

| Error | Cause | Resolution |
|-------|-------|------------|
| `Deserialization` | Invalid transaction format | Check compactc version compatibility |
| `Transaction` | Ledger validation failed | Verify proofs and balances |
| `BlockLimitExceededError` | TX too large for block | Split transaction or reduce operations |
| `FeeCalculationError` | Insufficient DUST | Fund wallet with DUST tokens |

### Transaction Validation Errors

```rust
pub enum TransactionError {
    InvalidProof,
    InsufficientBalance,
    ExpiredTTL,
    InvalidNetworkId,
    InvalidContractAddress,
    // ...
}
```

## Performance Considerations

### Proving Time

| Operation | Typical Duration |
|-----------|------------------|
| Deploy (simple) | 10-30 seconds |
| Circuit call | 5-15 seconds |
| Complex TX | 30-60+ seconds |

### Optimization Tips

1. Use remote proof server for faster proving
2. Batch multiple operations where possible
3. Pre-compute intents offline
4. Use appropriate TTL (10 minutes default) [10]

## See Also

- [util/toolkit](../util/toolkit/README.md) - Rust CLI toolkit
- [util/toolkit-js](../util/toolkit-js/README.md) - JavaScript toolkit
- [pallet-midnight](../pallets/midnight/README.md) - Core pallet documentation
- [ledger](../ledger/README.md) - Ledger integration
- [GLOSSARY](../GLOSSARY.md) - Term definitions
- [Compact Language Reference](https://docs.midnight.network/develop/reference/compact) - Official Compact documentation

---

## References

| # | Source | Path/URL |
|---|--------|----------|
| [1] | Midnight Official Documentation | https://docs.midnight.network |
| [2] | Midnight Toolkit README - Custom Contracts | `util/toolkit/README.md` (lines 261-293) |
| [3] | Toolkit-JS README - Configuration | `util/toolkit-js/README.md` (lines 1-77) |
| [4] | Midnight Toolkit README - Version Check | `util/toolkit/README.md` (lines 44-53) |
| [5] | Ledger Helpers - Contract Deploy | `ledger/helpers/src/versions/common/contract/deploy.rs` |
| [6] | Generate Intent Command | `util/toolkit/src/commands/generate_intent.rs` (lines 122-176) |
| [7] | Contract Deploy Builder | `util/toolkit/src/tx_generator/builder/builders/contract_deploy.rs` |
| [8] | Send Intent Command | `util/toolkit/src/commands/send_intent.rs` |
| [9] | Genesis Generator - Proof Server | `util/toolkit/src/genesis_generator.rs` (lines 304-342) |
| [10] | Transaction Builder | `ledger/helpers/src/versions/common/transaction.rs` (lines 163-223) |
| [11] | Destination Module - RPC URL | `util/toolkit/src/tx_generator/destination.rs` (line 24) |
| [12] | Sender Module - TX Submission | `util/toolkit/src/sender.rs` (lines 113-143) |
| [13] | Pallet Midnight - Extrinsics & Events | `pallets/midnight/src/lib.rs` (lines 347-412, 216-264) |
| [14] | Ledger State Management | `ledger/src/versions/common/mod.rs` (lines 331-393) |
| [15] | Pallet Midnight RPC | `pallets/midnight/rpc/src/lib.rs` (lines 32-49, 235-313) |
| [16] | Contract Maintenance | `ledger/helpers/src/versions/common/contract/maintenance.rs` |
