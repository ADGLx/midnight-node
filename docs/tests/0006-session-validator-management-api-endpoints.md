# Test Plan: D Parameter API Endpoint

**ADR:** [0006-session-validator-management-api-endpoints](../decisions/0006-session-validator-management-api-endpoints.md)
**Ticket:** [PM-20993](https://shielded.atlassian.net/browse/PM-20993)
**PR:** [#382](https://github.com/midnightntwrk/midnight-node/pull/382)

---

## Overview

This test plan validates the D Parameter API endpoint implemented in ADR-0006. The feature adds a Runtime API and RPC endpoint to expose D Parameter visibility for external tools and integrators.

Key changes validated:
1. [`DParameterProvider`](../../runtime/src/d_parameter.rs#L34) trait correctly abstracts D Parameter sourcing
2. [`get_d_parameter`](../../pallets/midnight/src/runtime_api.rs#L43) Runtime API exposes D Parameter to external callers
3. [`midnight_getDParameter`](../../pallets/midnight/rpc/src/lib.rs#L50) RPC endpoint provides JSON-RPC access

---

## Test Cases

| <div style="width:140px">Test ID</div> | <div style="width:300px">Objective</div> | <div style="width:350px">Steps</div> | <div style="width:300px">Expected Result</div> | <div style="width:50px">Type</div> |
|---|---|---|---|---|
| [PR382-TC-01](../../runtime/src/d_parameter.rs#L77) | Verify mock provider returns `None` to indicate inherent data should be used | 1. Call `MockDParameterProvider::get_d_parameter()`  <br>2. Verify return value is `None` | Returns `None`, no panics or errors | Unit |
| [PR382-TC-02](../../runtime/src/d_parameter.rs#L83) | Verify fixed provider returns the configured D Parameter values | 1. Define `FixedDParameterProvider<3, 2>`  <br>2. Call `get_d_parameter()`  <br>3. Verify return value | Returns `Some(DParameter)` with `num_permissioned_candidates = 3`, `num_registered_candidates = 2` | Unit |
| [PR382-TC-03](../../runtime/src/d_parameter.rs#L91) | Verify fixed provider handles zero values correctly | 1. Define `FixedDParameterProvider<0, 0>`  <br>2. Call `get_d_parameter()`  <br>3. Verify return value | Returns `Some(DParameter)` with both values = 0, no panics | Unit |
| [PR382-TC-04](../../runtime/src/d_parameter.rs#L99) | Verify fixed provider handles maximum u16 values correctly | 1. Define `FixedDParameterProvider<65535, 65535>`  <br>2. Call `get_d_parameter()`  <br>3. Verify return value | Returns `Some(DParameter)` with both values = 65535, no overflow | Unit |
| [PR382-TC-05](../../runtime/src/d_parameter.rs#L107) | Verify fixed provider works with permissioned-only configuration | 1. Define `FixedDParameterProvider<10, 0>`  <br>2. Call `get_d_parameter()`  <br>3. Verify return value | Returns `Some(DParameter)` with `num_permissioned = 10`, `num_registered = 0` | Unit |
| [PR382-TC-06](../../runtime/src/d_parameter.rs#L115) | Verify fixed provider works with registered-only configuration | 1. Define `FixedDParameterProvider<0, 5>`  <br>2. Call `get_d_parameter()`  <br>3. Verify return value | Returns `Some(DParameter)` with `num_permissioned = 0`, `num_registered = 5` | Unit |
| PR382-TC-07 | Verify Runtime API returns None when using MockDParameterProvider | 1. Start node with default config  <br>2. Query `MidnightRuntimeApi::get_d_parameter()` via RPC  <br>3. Verify return value | Returns `None` (JSON: `null`), API version is 6 | Integration |
| PR382-TC-08 | Verify RPC endpoint `midnight_getDParameter` works correctly | 1. Start node with `--dev` flag  <br>2. Call `midnight_getDParameter` via JSON-RPC  <br>3. Verify response | Returns `{"jsonrpc":"2.0","result":null,"id":1}`, no errors | Integration |
| PR382-TC-09 | Verify MidnightRuntimeApi version is 6 and `get_d_parameter` is callable | 1. Start node with `--dev` flag  <br>2. Query runtime metadata for `MidnightRuntimeApi` version  <br>3. Verify `get_d_parameter` method exists | API version is 6, method is present in metadata | Integration |

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

## Manual Verification Procedures

### Integration Tests (PR382-TC-07, PR382-TC-08, PR382-TC-09)

| Step | Action | Expected Outcome |
|------|--------|------------------|
| 1 | Build the node: `cargo build --release` | Build succeeds without errors |
| 2 | Start dev node: `./target/release/midnight-node --dev` | Node running on ws://127.0.0.1:9944 |
| 3 | Wait for block production | Logs show "Prepared block for proposing" |
| 4 | Query D Parameter (see command below) | Returns `{"jsonrpc":"2.0","result":null,"id":1}` |
| 5 | Check node logs | No errors related to D Parameter API |
| 6 | Stop node (Ctrl+C) | Clean shutdown |

**RPC Test Command:**

```bash
curl -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","method":"midnight_getDParameter","params":[],"id":1}' \
  http://127.0.0.1:9944
```

**Expected Response:**
```json
{"jsonrpc":"2.0","result":null,"id":1}
```

> **Note:** `result: null` is correct - the MockDParameterProvider returns `None`, indicating the D parameter from inherent data (main chain) should be used.

**API Version Verification (PR382-TC-09):**

```bash
# Query runtime metadata for API version
curl -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","method":"state_getMetadata","params":[],"id":1}' \
  http://127.0.0.1:9944 | jq '.result' | xxd -r -p | subxt metadata --format json | grep -A5 "MidnightRuntimeApi"
```

Alternatively, verify the `get_d_parameter` method exists by calling it successfully (covered by PR382-TC-08).
