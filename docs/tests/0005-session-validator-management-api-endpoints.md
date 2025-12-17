# Test Plan: D Parameter API Endpoint

**ADR:** [0005-session-validator-management-api-endpoints](../decisions/0005-session-validator-management-api-endpoints.md)
**Ticket:** [PM-20993](https://shielded.atlassian.net/browse/PM-20993)
**PR:** [#382](https://github.com/midnightntwrk/midnight-node/pull/382)

---

## Overview

This test plan validates the D Parameter API endpoint implemented in ADR-0005. The feature adds a Runtime API and RPC endpoint to expose D Parameter visibility for external tools and integrators.

---

## Scenarios Under Test

| Category | Scenario | Test Priority |
|----------|----------|---------------|
| **Provider** | Mock provider returns None | 🔴 HIGH |
| | Fixed provider returns configured values | 🔴 HIGH |
| | Fixed provider with zero values | 🟡 MEDIUM |
| | Fixed provider with max values | 🟡 MEDIUM |
| | Fixed provider permissioned only | 🟡 MEDIUM |
| | Fixed provider registered only | 🟡 MEDIUM |
| **Runtime API** | API returns None with mock provider | 🔴 HIGH |
| | API version is 6 | 🔴 HIGH |
| **RPC** | RPC endpoint returns correct response | 🔴 HIGH |
| | RPC handles missing API version gracefully | 🟡 MEDIUM |

---

## Test Cases

### PR382-TC-0005-01: MockDParameterProvider Returns None

**Objective:** Verify mock provider returns `None` to indicate inherent data should be used.

**Preconditions:**
- None

**Steps:**
1. Call `MockDParameterProvider::get_d_parameter()`
2. Verify return value

**Expected Result:**
- Returns `None`
- No panics or errors

**Success Criteria:** ✅ Mock provider correctly returns None for transition period

**Test Location:** `runtime/src/d_parameter.rs`

---

### PR382-TC-0005-02: FixedDParameterProvider Returns Configured Values

**Objective:** Verify fixed provider returns the configured D Parameter values.

**Preconditions:**
- None

**Steps:**
1. Define `FixedDParameterProvider<3, 2>`
2. Call `get_d_parameter()`
3. Verify return value

**Expected Result:**
- Returns `Some(DParameter { num_permissioned_candidates: 3, num_registered_candidates: 2 })`

**Success Criteria:** ✅ Fixed provider returns exact configured values

**Test Location:** `runtime/src/d_parameter.rs`

---

### PR382-TC-0005-03: FixedDParameterProvider With Zero Values

**Objective:** Verify fixed provider handles zero values correctly.

**Preconditions:**
- None

**Steps:**
1. Define `FixedDParameterProvider<0, 0>`
2. Call `get_d_parameter()`
3. Verify return value

**Expected Result:**
- Returns `Some(DParameter { num_permissioned_candidates: 0, num_registered_candidates: 0 })`
- No panics or errors

**Success Criteria:** ✅ Edge case: zero values handled correctly

**Test Location:** `runtime/src/d_parameter.rs`

---

### PR382-TC-0005-04: FixedDParameterProvider With Max Values

**Objective:** Verify fixed provider handles maximum u16 values correctly.

**Preconditions:**
- None

**Steps:**
1. Define `FixedDParameterProvider<65535, 65535>`
2. Call `get_d_parameter()`
3. Verify return value

**Expected Result:**
- Returns `Some(DParameter { num_permissioned_candidates: 65535, num_registered_candidates: 65535 })`
- No overflow or panics

**Success Criteria:** ✅ Edge case: max u16 values handled correctly

**Test Location:** `runtime/src/d_parameter.rs`

---

### PR382-TC-0005-05: FixedDParameterProvider Permissioned Only

**Objective:** Verify fixed provider works with permissioned-only configuration.

**Preconditions:**
- None

**Steps:**
1. Define `FixedDParameterProvider<10, 0>`
2. Call `get_d_parameter()`
3. Verify return value

**Expected Result:**
- Returns `Some(DParameter { num_permissioned_candidates: 10, num_registered_candidates: 0 })`

**Success Criteria:** ✅ Permissioned-only configuration works

**Test Location:** `runtime/src/d_parameter.rs`

---

### PR382-TC-0005-06: FixedDParameterProvider Registered Only

**Objective:** Verify fixed provider works with registered-only configuration.

**Preconditions:**
- None

**Steps:**
1. Define `FixedDParameterProvider<0, 5>`
2. Call `get_d_parameter()`
3. Verify return value

**Expected Result:**
- Returns `Some(DParameter { num_permissioned_candidates: 0, num_registered_candidates: 5 })`

**Success Criteria:** ✅ Registered-only configuration works

**Test Location:** `runtime/src/d_parameter.rs`

---

### PR382-TC-0005-07: Runtime API Returns None With Mock Provider

**Objective:** Verify Runtime API correctly returns None when using MockDParameterProvider.

**Preconditions:**
- Running node with default configuration

**Steps:**
1. Start node
2. Query `MidnightRuntimeApi::get_d_parameter()` via RPC
3. Verify return value

**Expected Result:**
- Returns `None` (JSON: `null`)
- API version is 6

**Success Criteria:** Runtime API integration works correctly with mock provider

**Test Location:** Integration test / Manual

---

### PR382-TC-0005-08: RPC Endpoint Returns Correct Response

**Objective:** Verify RPC endpoint `midnight_getDParameter` works correctly.

**Preconditions:**
- Running node on ws://127.0.0.1:9933

**Steps:**
1. Start node with `--dev` flag
2. Call `midnight_getDParameter` via JSON-RPC
3. Verify response

**Expected Result:**
- Returns `{ "jsonrpc": "2.0", "result": null, "id": 1 }`
- No errors

**Success Criteria:** RPC endpoint accessible and returns correct format

**Test Location:** Integration test / Manual

---

### PR382-TC-0005-09: API Version Is 6

**Objective:** Verify MidnightRuntimeApi version was incremented to 6.

**Preconditions:**
- None

**Steps:**
1. Check `pallets/midnight/src/runtime_api.rs`
2. Verify `#[api_version(6)]` annotation on trait
3. Verify `#[api_version(6)]` annotation on `get_d_parameter` method

**Expected Result:**
- API version is 6
- New method has `#[api_version(6)]` annotation

**Success Criteria:** ✅ API versioning correct for backward compatibility

**Test Location:** One-shot manual verification during code review

**Note:** This is a code review verification, not an automated test. Verified once during PR review.

---

## Test Matrix

| Test Case | Unit | Integration | E2E | Manual | Notes |
|-----------|------|-------------|-----|--------|-------|
| PR382-TC-0005-01 | ✅ | ➖ | ➖ | ➖ | `mock_provider_returns_none` |
| PR382-TC-0005-02 | ✅ | ➖ | ➖ | ➖ | `fixed_provider_returns_configured_values` |
| PR382-TC-0005-03 | ✅ | ➖ | ➖ | ➖ | `fixed_provider_with_zero_values` |
| PR382-TC-0005-04 | ✅ | ➖ | ➖ | ➖ | `fixed_provider_with_max_values` |
| PR382-TC-0005-05 | ✅ | ➖ | ➖ | ➖ | `fixed_provider_permissioned_only` |
| PR382-TC-0005-06 | ✅ | ➖ | ➖ | ➖ | `fixed_provider_registered_only` |
| PR382-TC-0005-07 | ➖ | ⏭️ | ➖ | ✅ | Requires running node |
| PR382-TC-0005-08 | ➖ | ⏭️ | ➖ | ✅ | Requires running node |
| PR382-TC-0005-09 | ➖ | ➖ | ➖ | ✅ | One-shot code review verification |

Legend: ⬜ Not Started | 🔄 In Progress | ✅ Pass | ❌ Fail | ⏭️ Skipped | ➖ N/A

---

## Running Tests

```bash
# Run unit tests
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

## Manual Testing Protocol

For validation requiring a running node:

| Step | Action | Expected Outcome |
|------|--------|------------------|
| 1 | Start dev node with `--dev` flag | Node running on ws://127.0.0.1:9933 |
| 2 | Call RPC: `{"jsonrpc":"2.0","method":"midnight_getDParameter","params":[],"id":1}` | Returns `{"result":null}` |
| 3 | Verify API version via `midnight_apiVersions` | Returns array including version info |
| 4 | Check node logs | No errors related to D Parameter API |

---

## References

- **ADR:** [0005-session-validator-management-api-endpoints](../decisions/0005-session-validator-management-api-endpoints.md)
- **Runtime API:** `pallets/midnight/src/runtime_api.rs` - `get_d_parameter`
- **RPC Implementation:** `pallets/midnight/rpc/src/lib.rs` - `midnight_getDParameter`
- **Provider Trait:** `runtime/src/d_parameter.rs` - `DParameterProvider`
