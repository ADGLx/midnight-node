# Session Validator Management API Endpoints

#### Status: Proposed
#### Date: 2025-12-17
#### Deciders: TBD
#### Jira: [PM-20993](https://shielded.atlassian.net/browse/PM-20993)

## Context and Problem Statement

### Problem

After PM-20994 completes (reusing the Partner Chains `pallet-session-validator-management` for validator selection), certain RPC and Runtime APIs from that pallet may expose misleading D parameter information.

Midnight has a **D-Parameter Override** mechanism (`DParameterOverride` storage in `pallet_midnight`) that can override the on-chain Cardano D parameter for emergency validator set management. However, the Partner Chains SDK's `SessionValidatorManagementApi` exposes information containing the **original** D parameter values, not the effective values used by the runtime.

### Impact

External tools and integrators calling these APIs would see:
- The on-chain D parameter policy ID, even when an override is active
- `calculate_committee()` results that differ from actual runtime behavior
- No way to determine if an override is active or what the effective D parameter is

### Technical Context

Current `SessionValidatorManagementApi` methods from Partner Chains:

| API Method | D-Parameter Issue |
|------------|------------------|
| `get_current_committee()` | ❌ No issue - returns stored committee |
| `get_next_committee()` | ❌ No issue - returns stored committee |
| `get_next_unset_epoch_number()` | ❌ No issue - returns epoch number |
| `calculate_committee(inputs, epoch)` | ⚠️ Caller passes `d_parameter`, but runtime applies override |
| `get_main_chain_scripts()` | ⚠️ Returns `d_parameter_policy_id` for on-chain value |

The `DParameterOverride` storage exists at `pallets/midnight/src/lib.rs` and is applied in `select_authorities_optionally_overriding()` at `runtime/src/lib.rs`.

## Considered Options

1. **Add new endpoints to `MidnightRuntimeApi`** - Extend existing Midnight-owned API
2. **Create a new dedicated pallet** - Separate "System Parameters" pallet
3. **Modify Partner Chains SDK** - Add override support upstream
4. **Do nothing** - Document the limitation

## Decision Outcome

**Chosen option: "Add new endpoints to `MidnightRuntimeApi`"** because:

- Follows existing patterns in the codebase
- `MidnightRuntimeApi` already exists and is well-versioned (v5)
- The `DParameterOverride` storage is in `pallet-midnight`, so colocation makes sense
- Midnight-owned, no dependency on external SDK changes
- Minimal new code, maximum reuse

## Consequences of the Options

### Option 1: Add new endpoints to `MidnightRuntimeApi` (Selected)

**Pros:**
- Follows existing patterns and architecture
- Reuses existing RPC infrastructure
- Fast to implement
- Midnight-controlled versioning

**Cons:**
- Slightly increases `pallet-midnight` scope

### Option 2: Create a new dedicated pallet

**Pros:**
- Clean separation of concerns
- Could aggregate multiple "system parameter" queries

**Cons:**
- Over-engineering for 2-3 new methods
- Additional pallet maintenance overhead
- New RPC module required

### Option 3: Modify Partner Chains SDK

**Pros:**
- Fixes the issue at the source
- Benefits all Partner Chains consumers

**Cons:**
- Not Midnight-controlled release cycle
- Slower turnaround
- May not be accepted upstream

### Option 4: Do nothing

**Pros:**
- No development effort

**Cons:**
- External integrators see incorrect data
- Override behavior is opaque to tooling
- Technical debt

## Technical Design

### New Runtime API Methods

Add to `pallets/midnight/src/runtime_api.rs`:

```rust
#[api_version(6)]
pub trait MidnightRuntimeApi {
    // ... existing methods ...
    
    /// Returns the current D parameter override, if set.
    /// Returns `None` if no override is active (using on-chain values).
    fn get_d_parameter_override() -> Option<(u16, u16)>;
    
    /// Returns the effective D parameter that will be used for authority selection.
    /// If an override is set, returns the override values.
    /// Otherwise, returns the provided on-chain values.
    fn get_effective_d_parameter(
        on_chain_num_permissioned: u16,
        on_chain_num_registered: u16
    ) -> (u16, u16);
}
```

### New RPC Endpoints

Add to `pallets/midnight/rpc/src/lib.rs`:

```rust
#[rpc(client, server)]
pub trait MidnightApi<BlockHash> {
    // ... existing methods ...
    
    #[method(name = "midnight_getDParameterOverride")]
    fn get_d_parameter_override(
        &self,
        at: Option<BlockHash>,
    ) -> RpcResult<Option<(u16, u16)>>;
    
    #[method(name = "midnight_getEffectiveDParameter")]
    fn get_effective_d_parameter(
        &self,
        on_chain_num_permissioned: u16,
        on_chain_num_registered: u16,
        at: Option<BlockHash>,
    ) -> RpcResult<(u16, u16)>;
}
```

### Runtime Implementation

Add to `runtime/src/lib.rs` in the `MidnightRuntimeApi` impl block:

```rust
fn get_d_parameter_override() -> Option<(u16, u16)> {
    pallet_midnight::pallet::DParameterOverride::<Runtime>::get()
}

fn get_effective_d_parameter(
    on_chain_num_permissioned: u16,
    on_chain_num_registered: u16
) -> (u16, u16) {
    match pallet_midnight::pallet::DParameterOverride::<Runtime>::get() {
        Some((override_perm, override_reg)) => (override_perm, override_reg),
        None => (on_chain_num_permissioned, on_chain_num_registered),
    }
}
```

## API Versioning

- Increment `MidnightRuntimeApi` from version 5 to version 6
- New methods only available in version 6+
- Existing methods remain backward compatible

## Testing Strategy

1. **Unit tests:**
   - `get_d_parameter_override()` returns `None` when no override set
   - `get_d_parameter_override()` returns `Some((x, y))` when override set
   - `get_effective_d_parameter()` returns on-chain values when no override
   - `get_effective_d_parameter()` returns override values when set

2. **Integration tests:**
   - Verify RPC endpoints return correct JSON-RPC responses

## Dependencies

- **Blocked by:** PM-20994 (Reuse PC pallet-session-validator-management)
- **No breaking changes** to existing APIs

## Decision Drivers

- Need for external tools to query effective validator selection parameters
- Existing `DParameterOverride` mechanism has no external visibility
- Follow established patterns in Midnight codebase
- Minimize new code and maintenance burden

