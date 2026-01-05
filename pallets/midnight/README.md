# pallet-midnight

Core [FRAME](https://docs.midnight.network/learn/glossary#frame-framework-for-runtime-aggregation-of-modularized-entities) pallet managing Midnight ledger state and transaction execution.

## Overview

This pallet is the primary interface between the Substrate runtime and the Midnight ledger. It processes privacy-preserving transactions, maintains the ledger state root, and emits events for contract operations. All Midnight transactions flow through this pallet's `send_mn_transaction` extrinsic.

The pallet implements `LedgerStateProviderMut` and `LedgerBlockContextProvider` traits, enabling other pallets to interact with ledger state.

## API Specification

### Dispatchables

| Call | Origin | Description |
|------|--------|-------------|
| `send_mn_transaction` | Unsigned | Process a Midnight transaction (ZSwap, contract deploy/call) |
| `override_d_parameter` | Root | Override validator selection D-parameter |
| `set_tx_size_weight` | Root | Configure transaction weight |

### Storage Items

| Name | Type | Description |
|------|------|-------------|
| `StateKey` | `BoundedVec<u8, 128>` | Current ledger state root |
| `NetworkId` | `BoundedVec<u8, 64>` | Network identifier (e.g., "undeployed") |
| `DParameterOverride` | `Option<(u16, u16)>` | Override for validator selection |
| `ConfigurableTransactionSizeWeight` | `Weight` | Transaction processing weight |

### Events

| Event | Description |
|-------|-------------|
| `ContractDeploy` | Contract deployed with address |
| `ContractCall` | Contract entrypoint invoked |
| `ContractMaintain` | Contract authority/verifier updated |
| `TxApplied` | Transaction fully applied |
| `TxPartialSuccess` | Guaranteed part applied, conditional failed |
| `UnshieldedTokens` | UTXO transfers (spent/created) |
| `PayoutMinted` | Block reward minted |
| `ClaimRewards` | Rewards claimed by beneficiary |

### Errors

| Error | Description |
|-------|-------------|
| `Transaction` | Ledger transaction validation/execution error |
| `Deserialization` | Failed to decode transaction |
| `NoLedgerState` | Ledger state not initialized |
| `BlockLimitExceededError` | Transaction exceeds block limits |

### Config Trait

| Associated Type | Description |
|-----------------|-------------|
| `BlockReward` | `Get<(u128, Option<Hash>)>` - Block reward amount and beneficiary |
| `SlotDuration` | Slot duration for timestamp calculations |

### Cargo Features

| Feature | Default | Description |
|---------|---------|-------------|
| `std` | Yes | Standard library support |
| `runtime-benchmarks` | No | Weight benchmarking |
| `try-runtime` | No | Migration testing |

## Architecture

```
Transaction Flow:
+------------------+     +------------------+     +------------------+
| send_mn_transaction | --> | LedgerApi::      | --> | Update StateKey  |
| (unsigned)       |     | apply_transaction|     | + Emit Events    |
+------------------+     +------------------+     +------------------+

Block Lifecycle:
+------------------+     +------------------+     +------------------+
| on_initialize    | --> | pre_fetch_storage| --> | Cache ledger     |
+------------------+     +------------------+     +------------------+
        |
        v (block execution - transactions processed)
        |
+------------------+     +------------------+     +------------------+
| on_finalize      | --> | post_block_update| --> | Mint rewards     |
+------------------+     | flush_storage    |     | Update state     |
+------------------+     +------------------+     +------------------+
```

**Sources**: [[1]](https://github.com/midnightntwrk/midnight-node/blob/main/pallets/midnight/src/lib.rs#L288-L306) [[2]](https://github.com/midnightntwrk/midnight-node/blob/main/pallets/midnight/src/lib.rs#L355)

## Usage

### Runtime Configuration

```rust
impl pallet_midnight::Config for Runtime {
    type BlockReward = LedgerBlockReward;
    type SlotDuration = ConstU64<SLOT_DURATION>;
}
```

### Querying State (via RPC)

RPC methods defined in `pallets/midnight/rpc/src/lib.rs`:

```rust
// RPC: midnight_contractState
fn get_state(contract_address: String, at: Option<BlockHash>) -> Result<String, StateRpcError>;

// RPC: midnight_zswapStateRoot  
fn get_zswap_state_root(at: Option<BlockHash>) -> Result<Vec<u8>, StateRpcError>;

// RPC: midnight_ledgerVersion
fn get_ledger_version(at: Option<BlockHash>) -> Result<String, BlockRpcError>;

// RPC: midnight_apiVersions
fn get_supported_api_versions() -> RpcResult<Vec<u32>>;
```

**Example curl usage:**
```bash
curl -X POST -H "Content-Type: application/json" \
  --data '{"jsonrpc":"2.0","method":"midnight_contractState","params":["<hex_address>"],"id":1}' \
  http://localhost:9944
```

## Integration

### Dependencies

- `midnight-node-ledger` - Ledger bridge API
- `midnight-primitives` - Shared types and traits
- `pallet-timestamp` - Block timestamp for context

### Used By

- `runtime` - Wired as primary transaction processor
- `pallet-midnight-rpc` - RPC interface to ledger queries
- `pallet-cnight-observation` - [System transaction](https://docs.midnight.network/learn/glossary#system-transaction) execution

## Testing

```bash
cargo test -p pallet-midnight
```

## See Also

- [pallet-midnight-rpc](rpc/README.md) - RPC interface
- [pallet-midnight-system](../midnight-system/README.md) - System transactions
- [ledger](../../ledger/README.md) - Ledger bridge implementation
