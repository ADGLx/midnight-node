# Session Validator Management API Endpoints

#### Status: Proposed
#### Date: 2025-12-17
#### Deciders: TBD
#### Jira: [PM-20993](https://shielded.atlassian.net/browse/PM-20993)

## Context and Problem Statement

### Problem

After PM-20994 completes, the D Parameter is sourced from a `DParameterProvider` trait rather than from Cardano D Parameter contracts. This trait is currently implemented by `MockDParameterProvider` (which returns `None`, meaning inherent data is used), but will be replaced by `pallet-system-parameters` when available.

External tools and integrators have no visibility into:
- Whether the D Parameter is governed on-chain or from inherent data
- What the on-chain D Parameter values are (when sourced from `pallet-system-parameters`)

### Technical Context

PM-20994 introduces a `DParameterProvider` trait in `runtime/src/d_parameter.rs`:

```rust
pub trait DParameterProvider {
    /// Returns the D Parameter to use for authority selection.
    /// Returns `Some(DParameter)` to use on-chain values,
    /// or `None` to use the inherent data value.
    fn get_d_parameter() -> Option<DParameter>;
}
```

Current implementations:
- `MockDParameterProvider` - Returns `None` (use inherent data during transition)
- `FixedDParameterProvider<P, R>` - Returns fixed values (testing only)
- Future: `pallet-system-parameters` integration

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

**Pros:**
- Follows existing patterns and architecture
- Reuses existing RPC infrastructure
- Fast to implement
- Ready for `pallet-system-parameters` when available

**Cons:**
- Currently returns `None` until real pallet is integrated

### Option 2: Wait for `pallet-system-parameters`

**Pros:**
- No interim implementation needed

**Cons:**
- Delays tooling integration
- No visibility during transition period

### Option 3: Do nothing

**Pros:**
- No development effort

**Cons:**
- D Parameter sourcing is opaque to external tools
- Technical debt

## Technical Design

### New Runtime API Methods

Add to `pallets/midnight/src/runtime_api.rs`:

```rust
#[api_version(6)]
pub trait MidnightRuntimeApi {
    // ... existing methods ...
    
    /// Returns the D Parameter from on-chain governance, if available.
    /// Returns `None` if D Parameter is sourced from inherent data.
    /// Returns `Some((num_permissioned, num_registered))` if sourced from
    /// `pallet-system-parameters`.
    fn get_d_parameter() -> Option<(u16, u16)>;
}
```

### New RPC Endpoint

Add to `pallets/midnight/rpc/src/lib.rs`:

```rust
#[rpc(client, server)]
pub trait MidnightApi<BlockHash> {
    // ... existing methods ...
    
    #[method(name = "midnight_getDParameter")]
    fn get_d_parameter(
        &self,
        at: Option<BlockHash>,
    ) -> RpcResult<Option<(u16, u16)>>;
}
```

### Runtime Implementation

Add to `runtime/src/lib.rs` in the `MidnightRuntimeApi` impl block:

```rust
fn get_d_parameter() -> Option<(u16, u16)> {
    use crate::d_parameter::{DParameterProvider, MockDParameterProvider};
    
    MockDParameterProvider::get_d_parameter()
        .map(|d| (d.num_permissioned_candidates, d.num_registered_candidates))
}
```

When `pallet-system-parameters` is integrated, this will change to:

```rust
fn get_d_parameter() -> Option<(u16, u16)> {
    use crate::d_parameter::{DParameterProvider, SystemParametersProvider};
    
    SystemParametersProvider::get_d_parameter()
        .map(|d| (d.num_permissioned_candidates, d.num_registered_candidates))
}
```

## API Versioning

- Increment `MidnightRuntimeApi` from version 5 to version 6
- New method only available in version 6+
- Existing methods remain backward compatible

## Testing Strategy

1. **Unit tests:**
   - `get_d_parameter()` returns `None` with `MockDParameterProvider`
   - `get_d_parameter()` returns `Some((P, R))` with `FixedDParameterProvider<P, R>`

2. **Integration tests:**
   - Verify RPC endpoint returns correct JSON-RPC response

## Dependencies

- **Blocked by:** PM-20994 (Aiken Permissioned Candidates / D Parameter Migration)
- **Future integration:** `pallet-system-parameters` (when available)
- **No breaking changes** to existing APIs

## Decision Drivers

- Need for external tools to query D Parameter source and values
- Prepare infrastructure for `pallet-system-parameters` integration
- Follow established patterns in Midnight codebase
- Minimize new code and maintenance burden
