# pallet-midnight

Core [FRAME](https://docs.midnight.network/learn/glossary#frame-framework-for-runtime-aggregation-of-modularized-entities) pallet managing Midnight ledger state and transaction execution.

## Overview

This pallet is the primary interface between the Substrate runtime and the Midnight ledger. It processes privacy-preserving transactions, maintains the ledger state root, and emits events for contract operations. All Midnight transactions flow through this pallet's `send_mn_transaction` extrinsic.

The pallet implements `LedgerStateProviderMut` and `LedgerBlockContextProvider` traits, enabling other pallets to interact with ledger state.

## API Specification

### Dispatchables

- [**`send_mn_transaction`**](https://github.com/midnightntwrk/midnight-node/blob/main/pallets/midnight/src/lib.rs#L288) - Process a Midnight transaction (ZSwap, contract deploy/call)
- [**`override_d_parameter`**](https://github.com/midnightntwrk/midnight-node/blob/main/pallets/midnight/src/lib.rs#L319) - Override validator selection D-parameter
- [**`set_tx_size_weight`**](https://github.com/midnightntwrk/midnight-node/blob/main/pallets/midnight/src/lib.rs#L333) - Configure transaction weight

### Storage Items

- [**`StateKey`**](https://github.com/midnightntwrk/midnight-node/blob/main/pallets/midnight/src/lib.rs#L183) - Current ledger state root
- [**`NetworkId`**](https://github.com/midnightntwrk/midnight-node/blob/main/pallets/midnight/src/lib.rs#L187) - Network identifier (e.g., "undeployed")
- [**`DParameterOverride`**](https://github.com/midnightntwrk/midnight-node/blob/main/pallets/midnight/src/lib.rs#L191) - Override for validator selection
- [**`ConfigurableTransactionSizeWeight`**](https://github.com/midnightntwrk/midnight-node/blob/main/pallets/midnight/src/lib.rs#L195) - Transaction processing weight

### Events

- [**`ContractDeploy`**](https://github.com/midnightntwrk/midnight-node/blob/main/pallets/midnight/src/lib.rs#L200) - Contract deployed with address
- [**`ContractCall`**](https://github.com/midnightntwrk/midnight-node/blob/main/pallets/midnight/src/lib.rs#L203) - Contract entrypoint invoked
- [**`ContractMaintain`**](https://github.com/midnightntwrk/midnight-node/blob/main/pallets/midnight/src/lib.rs#L206) - Contract authority/verifier updated
- [**`TxApplied`**](https://github.com/midnightntwrk/midnight-node/blob/main/pallets/midnight/src/lib.rs#L209) - Transaction fully applied
- [**`TxPartialSuccess`**](https://github.com/midnightntwrk/midnight-node/blob/main/pallets/midnight/src/lib.rs#L212) - Guaranteed part applied, conditional failed
- [**`UnshieldedTokens`**](https://github.com/midnightntwrk/midnight-node/blob/main/pallets/midnight/src/lib.rs#L215) - UTXO transfers (spent/created)
- [**`PayoutMinted`**](https://github.com/midnightntwrk/midnight-node/blob/main/pallets/midnight/src/lib.rs#L218) - Block reward minted
- [**`ClaimRewards`**](https://github.com/midnightntwrk/midnight-node/blob/main/pallets/midnight/src/lib.rs#L221) - Rewards claimed by beneficiary

### Errors

- [**`Transaction`**](https://github.com/midnightntwrk/midnight-node/blob/main/pallets/midnight/src/lib.rs#L226) - Ledger transaction validation/execution error
- [**`Deserialization`**](https://github.com/midnightntwrk/midnight-node/blob/main/pallets/midnight/src/lib.rs#L228) - Failed to decode transaction
- [**`NoLedgerState`**](https://github.com/midnightntwrk/midnight-node/blob/main/pallets/midnight/src/lib.rs#L230) - Ledger state not initialized
- [**`BlockLimitExceededError`**](https://github.com/midnightntwrk/midnight-node/blob/main/pallets/midnight/src/lib.rs#L232) - Transaction exceeds block limits

### Config Trait

- [**`BlockReward`**](https://github.com/midnightntwrk/midnight-node/blob/main/pallets/midnight/src/lib.rs#L166) - Block reward amount and beneficiary
- [**`SlotDuration`**](https://github.com/midnightntwrk/midnight-node/blob/main/pallets/midnight/src/lib.rs#L169) - Slot duration for timestamp calculations

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

### Querying State (via RPC)

```bash
# Get contract state
curl -X POST -H "Content-Type: application/json" \
  --data '{"jsonrpc":"2.0","method":"midnight_contractState","params":["<hex_address>"],"id":1}' \
  http://localhost:9944

# Get ledger version
curl -X POST -H "Content-Type: application/json" \
  --data '{"jsonrpc":"2.0","method":"midnight_ledgerVersion","params":[],"id":1}' \
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
