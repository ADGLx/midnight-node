# Session Validator Management API Endpoints

#### Status: Proposed
#### Date: 2025-12-17
#### Deciders: TBD
#### Jira: [PM-20993](https://shielded.atlassian.net/browse/PM-20993)

## Context and Problem Statement

### Problem

After PM-20994 completes, the D Parameter is sourced from a configurable provider rather than directly from Cardano D Parameter contracts. During the transition period, the provider returns inherent data. Eventually it will be replaced by `pallet-system-parameters`.

External tools and integrators have no visibility into:
- Whether the D Parameter is governed on-chain or from inherent data
- What the on-chain D Parameter values are (when sourced from `pallet-system-parameters`)

### Technical Context

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

1. **Add new endpoints to `MidnightRuntimeApi`** - Extend existing Midnight-owned API
2. **Wait for `pallet-system-parameters`** - Defer until the pallet is ready
3. **Do nothing** - Document the limitation

## Decision Outcome

**Chosen option: "Add new endpoints to `MidnightRuntimeApi`"** because:

- Provides immediate visibility into D Parameter sourcing
- Follows existing patterns in the codebase
- `MidnightRuntimeApi` already exists and is well-versioned (v5)
- Prepares infrastructure for `pallet-system-parameters` integration
- Minimal new code, maximum reuse

## Consequences of the Options

### Option 1: Add new endpoints to `MidnightRuntimeApi` (Selected)

- ✅ Follows existing patterns and architecture
- ✅ Reuses existing RPC infrastructure
- ✅ Fast to implement
- ✅ Ready for `pallet-system-parameters` when available
- ❌ Currently returns `None` until real pallet is integrated

### Option 2: Wait for `pallet-system-parameters`

- ✅ No interim implementation needed
- ❌ Delays tooling integration
- ❌ No visibility during transition period

### Option 3: Do nothing

- ✅ No development effort
- ❌ D Parameter sourcing is opaque to external tools
- ❌ Technical debt

## Notes

- **Blocked by:** PM-20994 (Aiken Permissioned Candidates / D Parameter Migration)
- **Future integration:** `pallet-system-parameters` (when available)
- No breaking changes to existing APIs
