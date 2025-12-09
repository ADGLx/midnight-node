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
- `NormalizedCost` represents costs as fractions of block limits (0.0 to 1.0)
- `overall_fullness` must be at least the maximum dimension per the [cost model spec](https://github.com/midnightntwrk/midnight-ledger/blob/ledger-6.2.0-rc.2/spec/cost-model.md)
- Using `unwrap_or(NormalizedCost::ZERO)` handles edge cases where block limits might be exceeded

### Solution 2: `pre_fetch` Migration

**Approach:** Use `key.hash()` instead of `&key` to get the `ArenaHash` reference.

**Implementation:** [`ledger/src/versions/common/mod.rs#L125`](../../../ledger/src/versions/common/mod.rs#L125)

```rust
// Old
backend.pre_fetch(&key, None, true)

// New
backend.pre_fetch(key.hash(), None, true)
```

**Rationale:** [`ArenaKey::hash()`](https://github.com/midnightntwrk/midnight-ledger/blob/ledger-6.2.0-rc.2/storage/src/arena.rs#L304) returns `&ArenaHash<H>`, which is now the required parameter type.

### Solution 3: `persist` Mutability

**Approach:** Change all `ledger` bindings to `mut ledger` where `persist()` is called.

**Implementation:** See changes in:
- [`ledger/src/versions/common/mod.rs`](../../../ledger/src/versions/common/mod.rs) (lines 154, 196, 283, 438)
- [`ledger/src/storage.rs#L49`](../../../ledger/src/storage.rs#L49)

**Rationale:** The upstream API now requires mutable access for persistence operations.

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
- Setting `overall_price` to ZERO makes all fees zero since `final_cost = overall_price * (...)` 
- Factors are set to ONE (multiplicative identity) rather than ZERO because:
  - Factors represent multipliers, where 1.0 is the neutral value
  - Zero factors could cause issues if accidentally used with [`update_from_fullness()`](https://github.com/midnightntwrk/midnight-ledger/blob/ledger-6.2.0-rc.2/base-crypto/src/cost_model.rs#L305)
  - Either approach works for genesis; ONE is more defensive

### Solution 5: `SerializableError` for `ArenaHash`

**Approach:** Added `ArenaHash` variant to [`SerializationError`](../../../ledger/src/versions/common/types.rs#L110) enum and implemented `SerializableError` trait for `ArenaHash<H>` in [`api/mod.rs#L80`](../../../ledger/src/versions/common/api/mod.rs#L80).

**Rationale:** `Sp::hash()` now returns `ArenaHash` which needs to be serializable for state root calculations.

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
| [`ledger/src/versions/common/mod.rs`](../../../ledger/src/versions/common/mod.rs) | Fixed `pre_fetch`, `persist` calls |
| [`ledger/src/versions/common/types.rs`](../../../ledger/src/versions/common/types.rs) | Added `ArenaHash` to `SerializationError` enum |
| [`ledger/src/storage.rs`](../../../ledger/src/storage.rs) | Fixed `persist` mutability |
| [`util/toolkit/src/genesis_generator.rs`](../../../util/toolkit/src/genesis_generator.rs) | Updated `post_block_update`, `FeePrices` |
| [`util/toolkit/src/commands/update_ledger_parameters.rs`](../../../util/toolkit/src/commands/update_ledger_parameters.rs) | Updated `FeePrices` fields |
| [`util/toolkit/src/commands/show_ledger_parameters.rs`](../../../util/toolkit/src/commands/show_ledger_parameters.rs) | Updated `FeePrices` fields |

## Related Documentation

- [midnight-ledger cost model specification](https://github.com/midnightntwrk/midnight-ledger/blob/ledger-6.2.0-rc.2/spec/cost-model.md)
- [`FeePrices::update_from_fullness()`](https://github.com/midnightntwrk/midnight-ledger/blob/ledger-6.2.0-rc.2/base-crypto/src/cost_model.rs#L305) for price adjustment algorithm
- [`SyntheticCost::normalize()`](https://github.com/midnightntwrk/midnight-ledger/blob/ledger-6.2.0-rc.2/base-crypto/src/cost_model.rs#L248) for cost normalization logic
