# runtime-common

Shared utilities for Midnight runtime configuration, focused on governance pallet integration.

## Overview

This crate provides adapter types that bridge governance pallets (`pallet_collective`, `pallet_membership`) with Substrate's account reference counting system. These utilities ensure that governance body members maintain proper account state (sufficients) so their accounts remain active even without holding balances.

## API Specification

### Public Types

- **`MembershipHandler<T, P>`** - Wrapper that manages account sufficients when membership changes
- **`MembershipObservationHandler<T, I>`** - Dispatches membership resets from inherent observations
- **`AlwaysNo`** - Default vote strategy that votes NO on abstentions

### MembershipHandler

Wraps an `InitializeMembers` + `ChangeMembers` implementation to automatically manage account sufficients.

**Behavior:**
- On `initialize_members`: Calls inner `P`, then increments sufficients for all members
- On `change_members_sorted`: Calls inner `P`, increments for incoming, decrements for outgoing

### MembershipObservationHandler

Bridges federated authority observations to `pallet_membership` instances.

**Implements:**
- `ChangeMembers<T::AccountId>` - Dispatches `reset_members` with `RawOrigin::None`
- `SortedMembers<T::AccountId>` - Reads current members from storage

### AlwaysNo

A `DefaultVote` implementation for `pallet_collective`. Ensures abstentions count as NO votes, requiring explicit approval for motions to pass.


## Integration

### Dependencies

- `frame-support` - FRAME traits (`ChangeMembers`, `InitializeMembers`)
- `frame-system` - Account sufficients management
- `pallet-collective` - `DefaultVote` trait
- `pallet-membership` - Member storage access

### Used By

- [`midnight-node-runtime`](https://github.com/midnightntwrk/midnight-node/blob/main/runtime/src/lib.rs) - Governance pallet configuration

## Why Sufficients?

Substrate accounts have three reference counters:
- **consumers** - Active uses (e.g., holding tokens)
- **providers** - Reasons account should exist (e.g., has balance)
- **sufficients** - Reasons account can exist without providers

Governance members need `sufficients > 0` so their accounts remain valid even if they hold no balance. `MembershipHandler` automates this bookkeeping.

## Testing

```bash
cargo test -p runtime-common
```

## See Also

- [runtime](../README.md) - Main runtime that uses these utilities
- [pallet-federated-authority-observation](../../pallets/federated-authority-observation/README.md) - Membership observation source

