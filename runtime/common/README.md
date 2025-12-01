# runtime-common

Shared utilities for Midnight runtime configuration, focused on governance pallet integration.

## Overview

This crate provides adapter types that bridge governance pallets (`pallet_collective`, `pallet_membership`) with Substrate's account reference counting system. These utilities ensure that governance body members maintain proper account state (sufficients) so their accounts remain active even without holding balances.

## API Specification

### Public Types

| Type | Description |
|------|-------------|
| `MembershipHandler<T, P>` | Wrapper that manages account sufficients when membership changes |
| `MembershipObservationHandler<T, I>` | Dispatches membership resets from inherent observations |
| `AlwaysNo` | Default vote strategy that votes NO on abstentions |

### MembershipHandler

Wraps an `InitializeMembers` + `ChangeMembers` implementation to automatically manage account sufficients:

```rust
pub struct MembershipHandler<T, P>(PhantomData<(T, P)>)
where
    T: frame_system::Config,
    P: InitializeMembers<T::AccountId> + ChangeMembers<T::AccountId>;
```

**Behavior:**
- On `initialize_members`: Calls inner `P`, then increments sufficients for all members
- On `change_members_sorted`: Calls inner `P`, increments for incoming, decrements for outgoing

### MembershipObservationHandler

Bridges federated authority observations to `pallet_membership` instances:

```rust
pub struct MembershipObservationHandler<T, I>(PhantomData<(T, I)>);
```

**Implements:**
- `ChangeMembers<T::AccountId>` - Dispatches `reset_members` with `RawOrigin::None`
- `SortedMembers<T::AccountId>` - Reads current members from storage

### AlwaysNo

A `DefaultVote` implementation for `pallet_collective`:

```rust
pub struct AlwaysNo;

impl DefaultVote for AlwaysNo {
    fn default_vote(...) -> bool { false }
}
```

Ensures abstentions count as NO votes, requiring explicit approval for motions to pass.

### Cargo Features

| Feature | Default | Description |
|---------|---------|-------------|
| `std` | Yes | Standard library support |

## Usage

### In Runtime Configuration

```rust
use runtime_common::governance::{AlwaysNo, MembershipHandler, MembershipObservationHandler};

// Configure pallet_collective with AlwaysNo default votes
impl pallet_collective::Config<CouncilCollectiveInstance> for Runtime {
    type DefaultVote = AlwaysNo;
    // ...
}

// Configure pallet_membership with MembershipHandler
impl pallet_membership::Config<CouncilMembershipInstance> for Runtime {
    type MembershipInitialized = MembershipHandler<Runtime, Council>;
    type MembershipChanged = MembershipHandler<Runtime, Council>;
    // ...
}

// Configure federated authority observation
impl pallet_federated_authority_observation::Config for Runtime {
    type CouncilMembershipHandler = MembershipObservationHandler<Runtime, CouncilMembershipInstance>;
    // ...
}
```

## Integration

### Dependencies

- `frame-support` - FRAME traits (`ChangeMembers`, `InitializeMembers`)
- `frame-system` - Account sufficients management
- `pallet-collective` - `DefaultVote` trait
- `pallet-membership` - Member storage access

### Used By

- `midnight-node-runtime` - Governance pallet configuration

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

