# Midnight-Ledger 6.2.0-rc.2 API Migration

#### Status: Accepted
#### Date: 2025-12-09
#### Deciders: TBD

## Context and Problem Statement

The `midnight-ledger` library is a core dependency that provides ledger state management, transaction processing, and cost model calculations. Version `6.2.0-rc.2` introduces several breaking API changes:

1. **`post_block_update` signature change** - Now requires normalized costs and overall block fullness as separate parameters
2. **`pre_fetch` parameter type change** - Accepts `ArenaHash` instead of `ArenaKey`
3. **`persist` mutability change** - Now requires `&mut self` instead of `&self`
4. **`FeePrices` structure redesign** - Changed from independent dimension prices to a single overall price with dimension-specific factors
5. **State key serialization** - Stricter type checking exposed an existing bug where the wrong type was being serialized

These changes enable more sophisticated fee adjustment algorithms and improve type safety, but require coordinated updates across the codebase.

## Decision Drivers

1. **Dependency alignment** - Keep midnight-node compatible with the latest ledger library
2. **Cost model improvements** - The new fee structure enables more flexible fee adjustment mechanisms
3. **Type safety** - Stricter type checking catches bugs earlier and improves correctness
4. **Blocking dependency** - Other planned features depend on ledger 6.2.0 capabilities

## Considered Options

### Option 1: Upgrade to 6.2.0-rc.2 and adapt to breaking changes (Selected)

Upgrade immediately and update all call sites to match the new API signatures.

- ✅ Enables access to new cost model features
- ✅ Fixes latent bug in state key serialization
- ✅ Aligns with upstream development
- ✅ Unblocks dependent features
- ❌ Requires coordinated changes across multiple crates
- ❌ Genesis files must be regenerated

### Option 2: Stay on current ledger version

Delay upgrade until a more convenient time.

- ✅ No immediate work required
- ❌ Blocks access to new features
- ❌ Increases technical debt as upstream diverges
- ❌ Latent serialization bug remains unfixed

### Option 3: Wait for stable 6.2.0 release

Wait for the release candidate period to complete.

- ✅ Avoids potential RC instability
- ❌ Unknown timeline for stable release
- ❌ Blocks dependent work
- ❌ May still require same migration effort

## Decision

Implement **Option 1: Upgrade to 6.2.0-rc.2 and adapt to breaking changes** because it enables access to the improved cost model, fixes a latent serialization bug, and unblocks dependent features.

Key constraints:
- Genesis files must be regenerated after code changes are applied
- Subxt metadata must be regenerated to match runtime changes
- CLI argument naming preserved for backward compatibility (semantic mismatch to be addressed separately)

## Confirmation

The decision will be validated through:

1. All existing tests continue to pass
2. Block production and finalization work correctly with new cost model
3. Genesis files load without state root mismatch errors
4. Toolkit commands work with regenerated metadata

## Notes

- The cost model changed from `sum(dimension_cost * dimension_price)` to `overall_price * sum(dimension_cost * dimension_factor)`
- CLI arguments in toolkit commands still use old naming (`read_price_a`, etc.) which may need future cleanup
- Branch: `bump-ledger-6.2`

## References

- [midnight-ledger 6.2.0-rc.2](https://github.com/midnightntwrk/midnight-ledger/tree/ledger-6.2.0-rc.2)
- [Cost model specification](https://github.com/midnightntwrk/midnight-ledger/blob/ledger-6.2.0-rc.2/spec/cost-model.md)
