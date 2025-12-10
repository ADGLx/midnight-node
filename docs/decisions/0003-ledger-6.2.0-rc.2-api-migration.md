# 3. Midnight-Ledger 6.2.0-rc.2 API Migration

**Date:** 2025-12-09  
**Status:** Accepted

**Sources:**
- Branch: `bump-ledger-6.2`
- midnight-ledger tag: [`ledger-6.2.0-rc.2`](https://github.com/midnightntwrk/midnight-ledger/tree/ledger-6.2.0-rc.2)

## Context and Problem Statement

The `midnight-ledger` dependency was upgraded to version `6.2.0-rc.2`, which introduced several breaking API changes that required updates throughout the codebase.

**Decision Drivers:**
* Keep midnight-node compatible with the latest ledger library
* Maintain backward compatibility where possible
* Ensure correct cost model calculations with the new fee structure

## Breaking Changes in midnight-ledger 6.2.0-rc.2

### 1. [`LedgerState::post_block_update`](https://github.com/midnightntwrk/midnight-ledger/blob/ledger-6.2.0-rc.2/ledger/src/semantics.rs#L1332) Signature Change

**Old signature:**
```rust
fn post_block_update(
    &self,
    tblock: Timestamp,
    block_fullness: SyntheticCost,
) -> Result<Self, BlockLimitExceeded>
```

**New signature:**
```rust
fn post_block_update(
    &self,
    tblock: Timestamp,
    detailed_block_fullness: NormalizedCost,
    overall_block_fullness: FixedPoint,
) -> Result<Self, BlockLimitExceeded>
```

### 2. [`StorageBackend::pre_fetch`](https://github.com/midnightntwrk/midnight-ledger/blob/ledger-6.2.0-rc.2/storage/src/backend.rs#L610) Parameter Type Change

**Old:** `pre_fetch(&ArenaKey<H>, ...)`  
**New:** `pre_fetch(&ArenaHash<H>, ...)`

### 3. [`Sp::persist`](https://github.com/midnightntwrk/midnight-ledger/blob/ledger-6.2.0-rc.2/storage/src/arena.rs#L1430) Mutability Change

**Old:** `fn persist(&self)`  
**New:** `fn persist(&mut self)`

### 4. [`FeePrices`](https://github.com/midnightntwrk/midnight-ledger/blob/ledger-6.2.0-rc.2/base-crypto/src/cost_model.rs#L276) Structure Redesign

**Old structure:**
```rust
pub struct FeePrices {
    pub read_price: FixedPoint,
    pub compute_price: FixedPoint,
    pub block_usage_price: FixedPoint,
    pub write_price: FixedPoint,
}
```

**New structure:**
```rust
pub struct FeePrices {
    pub overall_price: FixedPoint,
    pub read_factor: FixedPoint,
    pub compute_factor: FixedPoint,
    pub block_usage_factor: FixedPoint,
    pub write_factor: FixedPoint,
}
```

The cost model changed from independent dimension prices to a single `overall_price` with dimension-specific adjustment factors.

### 5. State Key Serialization Type Change

**Context:** The runtime expects state keys to be serialized as `TypedArenaKey<Ledger<D>, D::Hasher>` (tagged as `storage-key(ledger-state[v12])`), but the previous implementation serialized `ArenaHash` (tagged as `storage-hash`).

**Old:** `Sp::hash()` returns `ArenaHash<D::Hasher>` → serializes with tag `midnight:storage-hash:`  
**New:** `Sp::as_typed_key()` returns `TypedArenaKey<T, D::Hasher>` → serializes with tag `midnight:storage-key(ledger-state[v12]):`

This was not an upstream API change per se, but the stricter type checking in ledger-6.2 exposed an existing bug where the wrong type was being serialized. The [`Sp::as_typed_key()`](https://github.com/midnightntwrk/midnight-ledger/blob/ledger-6.2.0-rc.2/storage/src/arena.rs#L1412) method has always existed but was not being used.

## Decision Outcome

### Solution 1: `post_block_update` Migration

**Approach:** Convert `SyntheticCost` to `NormalizedCost` using block limits and compute `overall_block_fullness` as the maximum of all normalized dimensions.

**Implementation:** [`ledger/helpers/src/versions/common/context.rs#L138-L151`](../../../ledger/helpers/src/versions/common/context.rs#L138)

```rust
let block_limits = state.parameters.limits.block_limits;
let normalized_fullness = total_cost
    .normalize(block_limits)
    .unwrap_or(NormalizedCost::ZERO);
let overall_fullness = FixedPoint::max(
    FixedPoint::max(
        FixedPoint::max(normalized_fullness.read_time, normalized_fullness.compute_time),
        normalized_fullness.block_usage,
    ),
    FixedPoint::max(normalized_fullness.bytes_written, normalized_fullness.bytes_churned),
);
```

**Rationale:** 

The new `post_block_update` API separates detailed per-dimension costs from the overall block fullness metric, enabling more sophisticated fee adjustment algorithms. `NormalizedCost` represents each cost dimension as a fraction of its respective block limit (ranging from 0.0 to 1.0), which allows the ledger to reason about resource utilization independently of absolute values. This normalization is essential because the five dimensions (read time, compute time, block usage, bytes written, bytes churned) have different scales and units—normalization makes them directly comparable.

The `overall_block_fullness` parameter must capture the "bottleneck" dimension—the resource closest to its limit—because fee adjustment should respond to whichever resource is most constrained. Taking the maximum across all normalized dimensions is the mathematically correct approach per the [cost model specification](https://github.com/midnightntwrk/midnight-ledger/blob/ledger-6.2.0-rc.2/spec/cost-model.md). If we used an average or sum, a nearly-full block in one dimension could be masked by under-utilized dimensions, leading to inadequate fee responses. The defensive `unwrap_or(NormalizedCost::ZERO)` handles edge cases where block limits might be configured to zero or where division errors could occur, ensuring the node doesn't panic during block processing.

### Solution 2: `pre_fetch` Migration

**Approach:** Use `key.hash()` instead of `&key` to get the `ArenaHash` reference.

**Implementation:** [`ledger/src/versions/common/mod.rs#L125`](../../../ledger/src/versions/common/mod.rs#L125)

```rust
// Old
backend.pre_fetch(&key, None, true)

// New
backend.pre_fetch(key.hash(), None, true)
```

**Rationale:** 

The `pre_fetch` API change from `&ArenaKey<H>` to `&ArenaHash<H>` reflects an architectural refinement in how the storage layer identifies data. An `ArenaKey` is a composite type containing both a hash and type tag metadata, while `ArenaHash` is the pure content-addressable identifier. For pre-fetching operations, only the hash is needed to locate data on disk or in cache—the type tag is irrelevant at the storage layer since pre-fetching is about I/O optimization, not type-safe deserialization.

This change improves API clarity by accepting only the data actually needed and allows callers more flexibility (they may have a hash without a full key). The [`ArenaKey::hash()`](https://github.com/midnightntwrk/midnight-ledger/blob/ledger-6.2.0-rc.2/storage/src/arena.rs#L304) method provides zero-cost access to the inner `&ArenaHash<H>` reference, making migration straightforward with no performance impact.

### Solution 3: `persist` Mutability

**Approach:** Change all `ledger` bindings to `mut ledger` where `persist()` is called.

**Implementation:** See changes in:
- [`ledger/src/versions/common/mod.rs`](../../../ledger/src/versions/common/mod.rs) (lines 154, 196, 283, 438)
- [`ledger/src/storage.rs#L49`](../../../ledger/src/storage.rs#L49)

**Rationale:** 

The change from `&self` to `&mut self` for `persist()` reflects a more accurate modeling of the persistence operation's semantics. While the previous `&self` signature suggested persistence was a pure read operation, in reality `persist()` may update internal state such as marking data as persisted, updating cache metadata, clearing dirty flags, or modifying write-ahead log pointers. The `&mut self` signature makes these potential side effects explicit in the type system, preventing accidental concurrent persistence calls and enabling the compiler to enforce single-writer semantics.

This is a correctness improvement: Rust's ownership system now guarantees that no other code can hold references to the state being persisted, eliminating a class of potential race conditions. The required changes are mechanical—adding `mut` to bindings—and have no runtime cost. The locations where `persist()` is called are already exclusive owners of the state (not shared references), so the logical ownership was always mutable; the signature now accurately reflects this.

### Solution 4: `FeePrices` Field Mapping

**Approach:** Map old field names to new field names:
- `read_price` → `read_factor`
- `compute_price` → `compute_factor`
- `block_usage_price` → `block_usage_factor`
- `write_price` → `write_factor`
- Added: `overall_price` (new base price field)

**For [`without_fees`](../../../util/toolkit/src/genesis_generator.rs#L564) function:**

```rust
FeePrices {
    overall_price: FixedPoint::ZERO,
    read_factor: FixedPoint::ONE,
    compute_factor: FixedPoint::ONE,
    block_usage_factor: FixedPoint::ONE,
    write_factor: FixedPoint::ONE,
}
```

**Rationale:** 

The `FeePrices` restructuring represents a fundamental shift in how the cost model computes fees. Previously, each dimension had an independent price, and the final fee was effectively `sum(dimension_cost * dimension_price)`. The new model uses `overall_price * sum(dimension_cost * dimension_factor)`, separating the base price level from the relative weighting of dimensions. This enables cleaner fee adjustment: `overall_price` can scale with network demand while `*_factor` weights remain stable, or vice versa.

For the `without_fees` genesis configuration, setting `overall_price` to `ZERO` guarantees zero fees regardless of other values since it's a multiplicative term in the fee formula. The factors are set to `ONE` (the multiplicative identity) rather than `ZERO` for defensive programming: while either achieves zero fees when `overall_price` is zero, factors of ONE are semantically correct ("no adjustment") and avoid potential division-by-zero or unexpected behavior if [`update_from_fullness()`](https://github.com/midnightntwrk/midnight-ledger/blob/ledger-6.2.0-rc.2/base-crypto/src/cost_model.rs#L305) is ever invoked during testing. This choice reflects the principle that default values should be neutral/identity values for their mathematical role.

### Solution 5: `SerializableError` for `ArenaHash`

**Approach:** Added `ArenaHash` variant to [`SerializationError`](../../../ledger/src/versions/common/types.rs#L110) enum and implemented `SerializableError` trait for `ArenaHash<H>` in [`api/mod.rs#L80`](../../../ledger/src/versions/common/api/mod.rs#L80).

**Rationale:** 

The `SerializableError` trait is required for types that participate in the storage arena's error handling during serialization and deserialization operations. With the upstream API changes, `ArenaHash<H>` is now used directly in more contexts (such as the new `pre_fetch` signature), and the trait bounds require it to implement `SerializableError` so that failures can be properly typed and propagated.

Adding the `ArenaHash` variant to midnight-node's `SerializationError` enum follows the existing pattern established for other arena types (`TypedArenaKey`, `ArenaKey`). The implementation returns a distinct error variant, enabling precise error discrimination when debugging serialization failures. This is the minimal, correct solution: it satisfies the trait bound without changing error handling semantics, and the new variant integrates naturally with the existing `Display` and `Debug` implementations for `SerializationError`.

### Solution 6: State Key Serialization (`Sp::hash()` → `Sp::as_typed_key()`)

**Approach:** Use `Sp::as_typed_key()` instead of `Sp::hash()` when serializing the state key for storage.

**Implementation:** 
- [`ledger/src/storage.rs#L32,L53`](../../../ledger/src/storage.rs#L32) - Genesis initialization
- [`ledger/src/versions/common/mod.rs#L162`](../../../ledger/src/versions/common/mod.rs#L162) - `post_block_update()`
- [`ledger/src/versions/common/mod.rs#L218`](../../../ledger/src/versions/common/mod.rs#L218) - `apply_transaction()`
- [`ledger/src/versions/common/mod.rs#L286`](../../../ledger/src/versions/common/mod.rs#L286) - `apply_system_transaction()`
- [`ledger/src/versions/common/mod.rs#L442`](../../../ledger/src/versions/common/mod.rs#L442) - `mint_coins()`

```rust
// Old (incorrect for ledger-6.2)
api.tagged_serialize(&ledger.hash())

// New (correct)
api.tagged_serialize(&ledger.as_typed_key())
```

**Rationale:**

The runtime's `pre_fetch_storage()` and `get_ledger()` functions expect the state key to be a serialized `TypedArenaKey<Ledger<D>, D::Hasher>`, which deserializes with the tag `midnight:storage-key(ledger-state[v12]):`. However, `Sp::hash()` returns an `ArenaHash<D::Hasher>`, which serializes with the tag `midnight:storage-hash:`.

This mismatch caused runtime panics during block production:
```
Error deserializing: "...TypedArenaKey...": Custom { kind: InvalidData, 
  error: "expected header tag 'midnight:storage-key(ledger-state[v12]):', 
          got 'midnight:storage-hash:...'" }
```

The [`Sp::as_typed_key()`](https://github.com/midnightntwrk/midnight-ledger/blob/ledger-6.2.0-rc.2/storage/src/arena.rs#L1412) method returns the correctly typed `TypedArenaKey<T, D::Hasher>` that wraps the underlying `ArenaKey` with proper type information. This ensures the serialized state key can be deserialized by the ledger API functions that expect `TypedArenaKey`.

This change affects:
- Genesis initialization (`get_root()` and `alloc_with_initial_state()`)
- Post-block state updates (`post_block_update()`)
- Transaction processing (`apply_transaction()`, `apply_system_transaction()`)
- Coin minting (`mint_coins()`)

### Solution 7: Genesis File Regeneration

**Approach:** Regenerate all genesis files (`res/genesis/*.mn`) using the updated toolkit after the `as_typed_key()` fix is applied.

**Implementation:** Run the genesis regeneration for each network:
```bash
midnight-node-toolkit generate-genesis --network undeployed --seeds-file <seeds.json>
```

**Rationale:**

Genesis files are binary serializations of the initial ledger state and transactions, created by the toolkit and embedded in the node binary at compile time. After the `as_typed_key()` fix, any genesis files generated with the old code contain state keys serialized with `storage-hash:` tags. When the node starts with these old genesis files but uses the new runtime code, a state root mismatch occurs:

```
Error: PanicError("Ledger state root mismatch: expected ...storage-key(ledger-state[v12]):..., 
                   actual ...storage-hash:...")
```

The genesis block (block 0) writes its state key in the old format, but block 1+ writes state keys in the new format, causing deserialization failures when the toolkit or runtime tries to verify state consistency.

**Critical:** Genesis files MUST be regenerated AFTER the code changes are merged, not before. The regeneration order is:
1. Apply `as_typed_key()` code changes
2. Rebuild the toolkit
3. Regenerate genesis files with the new toolkit
4. Regenerate metadata with `/bot rebuild-metadata`

### Solution 8: Metadata Regeneration

**Approach:** Regenerate Subxt metadata after runtime changes using `/bot rebuild-metadata` on the PR.

**Rationale:**

The `FeePrices` structure change and other runtime modifications alter the node's metadata, which Subxt uses for type-safe RPC communication. The toolkit uses Subxt to communicate with the node, and if the compiled Subxt bindings don't match the node's metadata, you get:

```
Error: SubxtError(Metadata(IncompatibleCodegen))
```

The metadata regeneration extracts fresh metadata from a running node built with the new code and updates `metadata/static/midnight_metadata.scale`.

## Consequences

**Positive:**
* Codebase is compatible with midnight-ledger 6.2.0-rc.2
* New cost model provides more flexible fee adjustment mechanisms
* Normalized costs allow better block limit enforcement

**Negative:**
* CLI arguments in toolkit commands still use old naming (`read_price_a`, etc.) which may be confusing
* The fee price semantic change (prices → factors) requires documentation updates

**Risks:**
* The mapping of old CLI args to new factor fields preserves functionality but may need revisiting for semantic correctness

## Files Changed

| File | Changes |
|------|---------|
| [`ledger/helpers/src/versions/common/mod.rs`](../../../ledger/helpers/src/versions/common/mod.rs) | Added `NormalizedCost` export |
| [`ledger/helpers/src/versions/common/context.rs`](../../../ledger/helpers/src/versions/common/context.rs) | Updated `post_block_update` call |
| [`ledger/src/versions/common/api/ledger.rs`](../../../ledger/src/versions/common/api/ledger.rs) | Updated `post_block_update`, imports |
| [`ledger/src/versions/common/api/mod.rs`](../../../ledger/src/versions/common/api/mod.rs) | Added `ArenaHash` import, `SerializableError` impl |
| [`ledger/src/versions/common/mod.rs`](../../../ledger/src/versions/common/mod.rs) | Fixed `pre_fetch`, `persist` calls, changed `hash()` to `as_typed_key()` in 4 functions |
| [`ledger/src/versions/common/types.rs`](../../../ledger/src/versions/common/types.rs) | Added `ArenaHash` to `SerializationError` enum |
| [`ledger/src/storage.rs`](../../../ledger/src/storage.rs) | Fixed `persist` mutability, changed `hash()` to `as_typed_key()` |
| [`util/toolkit/src/genesis_generator.rs`](../../../util/toolkit/src/genesis_generator.rs) | Updated `post_block_update`, `FeePrices` |
| [`util/toolkit/src/commands/update_ledger_parameters.rs`](../../../util/toolkit/src/commands/update_ledger_parameters.rs) | Updated `FeePrices` fields |
| [`util/toolkit/src/commands/show_ledger_parameters.rs`](../../../util/toolkit/src/commands/show_ledger_parameters.rs) | Updated `FeePrices` fields |
| [`res/genesis/genesis_block_undeployed.mn`](../../../res/genesis/genesis_block_undeployed.mn) | Regenerated with `as_typed_key()` state key format |
| [`res/genesis/genesis_state_undeployed.mn`](../../../res/genesis/genesis_state_undeployed.mn) | Regenerated with `as_typed_key()` state key format |
| [`metadata/static/midnight_metadata.scale`](../../../metadata/static/midnight_metadata.scale) | Regenerated to match updated runtime |

## Related Documentation

- [midnight-ledger cost model specification](https://github.com/midnightntwrk/midnight-ledger/blob/ledger-6.2.0-rc.2/spec/cost-model.md)
- [`FeePrices::update_from_fullness()`](https://github.com/midnightntwrk/midnight-ledger/blob/ledger-6.2.0-rc.2/base-crypto/src/cost_model.rs#L305) for price adjustment algorithm
- [`SyntheticCost::normalize()`](https://github.com/midnightntwrk/midnight-ledger/blob/ledger-6.2.0-rc.2/base-crypto/src/cost_model.rs#L248) for cost normalization logic
