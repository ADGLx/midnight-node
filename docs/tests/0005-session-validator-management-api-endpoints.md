# Test Plan: Session Validator Management API Endpoints

**ADR:** [ADR-0005](../decisions/0005-session-validator-management-api-endpoints.md)
**PR:** [#382](https://github.com/midnightntwrk/midnight-node/pull/382)
**Ticket:** [PM-20993](https://shielded.atlassian.net/browse/PM-20993)

---

## Overview

This test plan covers the new Runtime API and RPC endpoints for session validator management in `pallet-midnight`.

---

## Test Matrix

| Test Case | Description | Unit | Integration | E2E |
|-----------|-------------|:----:|:-----------:|:---:|
| PR382-TC-0005-01 | `get_d_parameter_override()` returns None when no override set | | | |
| PR382-TC-0005-02 | `get_d_parameter_override()` returns override when set | | | |
| PR382-TC-0005-03 | `get_effective_d_parameter()` returns inherent D param when no override | | | |
| PR382-TC-0005-04 | `get_effective_d_parameter()` returns overridden D param when override set | | | |
| PR382-TC-0005-05 | RPC endpoint `midnight_getDParameterOverride` works correctly | | | |
| PR382-TC-0005-06 | RPC endpoint `midnight_getEffectiveDParameter` works correctly | | | |
| PR382-TC-0005-07 | API version incremented to v6 | | | |

---

## Test Details

### PR382-TC-0005-01: No override returns None

**Given:** No D parameter override has been set
**When:** `get_d_parameter_override()` is called
**Then:** Returns `None`

### PR382-TC-0005-02: Override returns value

**Given:** D parameter override is set to (P, R)
**When:** `get_d_parameter_override()` is called
**Then:** Returns `Some((P, R))`

### PR382-TC-0005-03: Effective D param without override

**Given:** No D parameter override is set
**When:** `get_effective_d_parameter()` is called
**Then:** Returns the D parameter from inherent data

### PR382-TC-0005-04: Effective D param with override

**Given:** D parameter override is set to (P, R)
**When:** `get_effective_d_parameter()` is called
**Then:** Returns the override value (P, R)

### PR382-TC-0005-05: RPC getDParameterOverride

**Given:** RPC server is running
**When:** `midnight_getDParameterOverride` RPC is called
**Then:** Returns correct override status

### PR382-TC-0005-06: RPC getEffectiveDParameter

**Given:** RPC server is running
**When:** `midnight_getEffectiveDParameter` RPC is called
**Then:** Returns effective D parameter

### PR382-TC-0005-07: API version check

**Given:** Runtime is built
**When:** `MidnightRuntimeApi` version is queried
**Then:** Returns version 6

---

## Notes

- Test plan to be updated as implementation progresses

