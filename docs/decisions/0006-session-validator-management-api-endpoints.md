# Session Validator Management API Endpoints

#### Status: Proposed
#### Date: 2025-12-17
#### Deciders: TBD
#### Jira: [PM-20993](https://shielded.atlassian.net/browse/PM-20993)

## Context and Problem Statement

After PM-20994 completes, the D Parameter is sourced from a configurable provider rather than directly from Cardano D Parameter contracts. During the transition period, the provider returns inherent data. Eventually it will be replaced by `pallet-system-parameters`.

External tools and integrators have no visibility into:
- Whether the D Parameter is governed on-chain or from inherent data
- What the on-chain D Parameter values are (when sourced from `pallet-system-parameters`)

PM-20994 requires that the D Parameter be abstracted behind a provider interface rather than read directly from Cardano contracts. This enables:
- A mock provider during the transition period (returns inherent data)
- Future integration with `pallet-system-parameters` when available

The provider may return on-chain values or signal that inherent data should be used instead.

## Decision Drivers

1. **External visibility** - External tools need to query D Parameter source and values
2. **Future readiness** - Prepare infrastructure for `pallet-system-parameters` integration
3. **Consistency** - Follow established patterns in Midnight codebase
4. **Minimal effort** - Minimize new code and maintenance burden

## Considered Options

### Option 1: Add new endpoints to `MidnightRuntimeApi` (Selected)

Extend the existing Midnight-owned Runtime API to expose D Parameter information.

- ✅ Follows existing patterns and architecture
- ✅ Reuses existing RPC infrastructure
- ✅ Fast to implement
- ✅ Ready for `pallet-system-parameters` when available
- ❌ Currently returns `None` until real pallet is integrated

### Option 2: Wait for `pallet-system-parameters`

Defer API work until the pallet is ready.

- ✅ No interim implementation needed
- ❌ Delays tooling integration
- ❌ No visibility during transition period

### Option 3: Do nothing

Document the limitation and leave D Parameter opaque to external tools.

- ✅ No development effort
- ❌ D Parameter sourcing is opaque to external tools
- ❌ Technical debt

## Decision

Implement **Option 1: Add new endpoints to `MidnightRuntimeApi`** because it provides immediate visibility into D Parameter sourcing, follows existing patterns in the codebase, and prepares infrastructure for `pallet-system-parameters` integration with minimal new code.

Key constraints:
- `MidnightRuntimeApi` already exists and is well-versioned (v5)
- No breaking changes to existing APIs

## Confirmation

The decision will be validated through:

1. RPC endpoint returns correct response format
2. No regression in existing `MidnightRuntimeApi` functionality
3. Documentation updated for new endpoints

## Notes

- **Blocked by:** PM-20994 (Aiken Permissioned Candidates / D Parameter Migration)
- **Future integration:** `pallet-system-parameters` (when available)

## References

- [ADR-0005: Aiken Permissioned Candidates & D Parameter Migration](https://github.com/midnightntwrk/midnight-node/blob/feat/aiken-permissioned-candidates-migration/docs/decisions/0005-aiken-permissioned-candidates-d-parameter-migration.md) (PR #378)
