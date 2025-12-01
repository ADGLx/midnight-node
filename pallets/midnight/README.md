# pallet-midnight

Core [FRAME](../../GLOSSARY.md#frame-framework-for-runtime-aggregation-of-modularized-entities) pallet managing Midnight ledger state and transaction execution.

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

## Usage

### Runtime Configuration

```rust
impl pallet_midnight::Config for Runtime {
    type BlockReward = LedgerBlockReward;
    type SlotDuration = ConstU64<SLOT_DURATION>;
}
```

### Querying State (via Runtime API)

```rust
// Get contract state
let state = Midnight::get_contract_state(&contract_address)?;

// Get network ID
let network = Midnight::get_network_id();

// Get ledger version
let version = Midnight::get_ledger_version();
```

## Integration

### Dependencies

- `midnight-node-ledger` - Ledger bridge API
- `midnight-primitives` - Shared types and traits
- `pallet-timestamp` - Block timestamp for context

### Used By

- `runtime` - Wired as primary transaction processor
- `pallet-midnight-rpc` - RPC interface to ledger queries
- `pallet-cnight-observation` - [System transaction](../../GLOSSARY.md#system-transaction) execution

## Testing

```bash
cargo test -p pallet-midnight
```

## See Also

- [pallet-midnight-rpc](rpc/README.md) - RPC interface
- [pallet-midnight-system](../midnight-system/README.md) - System transactions
- [ledger](../../ledger/README.md) - Ledger bridge implementation
