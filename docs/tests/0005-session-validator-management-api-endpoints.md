# Test Plan: Session Validator Management API Endpoints

**ADR:** [ADR-0005](../decisions/0005-session-validator-management-api-endpoints.md)
**PR:** [#382](https://github.com/midnightntwrk/midnight-node/pull/382)
**Ticket:** [PM-20993](https://shielded.atlassian.net/browse/PM-20993)

---

## Overview

This test plan covers the new Runtime API and RPC endpoint for querying D Parameter information from `pallet-system-parameters` (when available).

---

## Test Matrix

| Test Case | Description | Unit | Integration | E2E |
|-----------|-------------|:----:|:-----------:|:---:|
| PR382-TC-0005-01 | `MockDParameterProvider::get_d_parameter()` returns `None` | ✓ | | |
| PR382-TC-0005-02 | `FixedDParameterProvider<P,R>::get_d_parameter()` returns `Some((P,R))` | ✓ | | |
| PR382-TC-0005-03 | `FixedDParameterProvider` with zero values | ✓ | | |
| PR382-TC-0005-04 | `FixedDParameterProvider` with max u16 values | ✓ | | |
| PR382-TC-0005-05 | `FixedDParameterProvider` permissioned only (P>0, R=0) | ✓ | | |
| PR382-TC-0005-06 | `FixedDParameterProvider` registered only (P=0, R>0) | ✓ | | |
| PR382-TC-0005-07 | Runtime API `get_d_parameter()` returns `None` with mock provider | | ✓ | |
| PR382-TC-0005-08 | RPC `midnight_getDParameter` returns correct response | | ✓ | |
| PR382-TC-0005-09 | API version incremented to v6 | ✓ | | |

---

## Test Details

### PR382-TC-0005-01: MockDParameterProvider returns None

**Given:** Using `MockDParameterProvider`
**When:** `get_d_parameter()` is called
**Then:** Returns `None` (use inherent data)

**Status:** ✅ Implemented in `runtime/src/d_parameter.rs`

### PR382-TC-0005-02: FixedDParameterProvider returns configured values

**Given:** Using `FixedDParameterProvider<3, 2>`
**When:** `get_d_parameter()` is called
**Then:** Returns `Some(DParameter { num_permissioned_candidates: 3, num_registered_candidates: 2 })`

**Status:** ✅ Implemented in `runtime/src/d_parameter.rs`

### PR382-TC-0005-03: FixedDParameterProvider with zero values

**Given:** Using `FixedDParameterProvider<0, 0>`
**When:** `get_d_parameter()` is called
**Then:** Returns `Some(DParameter { num_permissioned_candidates: 0, num_registered_candidates: 0 })`

**Status:** ✅ Implemented in `runtime/src/d_parameter.rs`

### PR382-TC-0005-04: FixedDParameterProvider with max values

**Given:** Using `FixedDParameterProvider<65535, 65535>`
**When:** `get_d_parameter()` is called
**Then:** Returns `Some(DParameter { num_permissioned_candidates: 65535, num_registered_candidates: 65535 })`

**Status:** ✅ Implemented in `runtime/src/d_parameter.rs`

### PR382-TC-0005-05: FixedDParameterProvider permissioned only

**Given:** Using `FixedDParameterProvider<10, 0>`
**When:** `get_d_parameter()` is called
**Then:** Returns `Some(DParameter { num_permissioned_candidates: 10, num_registered_candidates: 0 })`

**Status:** ✅ Implemented in `runtime/src/d_parameter.rs`

### PR382-TC-0005-06: FixedDParameterProvider registered only

**Given:** Using `FixedDParameterProvider<0, 5>`
**When:** `get_d_parameter()` is called
**Then:** Returns `Some(DParameter { num_permissioned_candidates: 0, num_registered_candidates: 5 })`

**Status:** ✅ Implemented in `runtime/src/d_parameter.rs`

### PR382-TC-0005-07: Runtime API with mock provider

**Given:** Runtime using `MockDParameterProvider`
**When:** `MidnightRuntimeApi::get_d_parameter()` is called
**Then:** Returns `None`

**Status:** Implementation complete, integration test requires running node

### PR382-TC-0005-08: RPC endpoint returns correct response

**Given:** RPC server is running
**When:** `midnight_getDParameter` RPC is called
**Then:** Returns `null` (JSON representation of `None`)

**Status:** Implementation complete, integration test requires running node

### PR382-TC-0005-09: API version check

**Given:** Runtime is built
**When:** `MidnightRuntimeApi` version is queried
**Then:** Returns version 6

**Status:** ✅ Verified in `pallets/midnight/src/runtime_api.rs` (`#[api_version(6)]`)

---

## Implementation Summary

### Files Changed

- `pallets/midnight/src/runtime_api.rs` - Added `get_d_parameter()` API method, version bump to 6
- `pallets/midnight/rpc/src/lib.rs` - Added `midnight_getDParameter` RPC endpoint
- `runtime/src/d_parameter.rs` - New module with `DParameterProvider` trait and implementations
- `runtime/src/lib.rs` - Added module declaration and Runtime API implementation

### Test Commands

```bash
# Run d_parameter unit tests
cargo test -p midnight-node-runtime d_parameter

# Expected output: 6 tests pass
# - mock_provider_returns_none
# - fixed_provider_returns_configured_values
# - fixed_provider_with_zero_values
# - fixed_provider_with_max_values
# - fixed_provider_permissioned_only
# - fixed_provider_registered_only

# Check builds
cargo check -p pallet-midnight-rpc
cargo check -p midnight-node-runtime
```

---

## Notes

- Currently uses `MockDParameterProvider` which returns `None` (use inherent data)
- Will be updated to use `pallet-system-parameters` when available
- Blocked by PM-20994 for the `DParameterProvider` trait infrastructure
- Integration tests (TC-0005-07, TC-0005-08) require a running node and are deferred to manual testing
