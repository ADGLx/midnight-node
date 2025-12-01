# midnight-primitives-federated-authority-observation

Shared types for observing [federated authority](../../GLOSSARY.md#federated-authority) membership from Cardano.

## Overview

This crate defines types for synchronizing governance body membership ([Council](../../GLOSSARY.md#council), [Technical Committee](../../GLOSSARY.md#technical-committee)) from Cardano to Midnight. Membership changes observed on Cardano are propagated via inherents to update the corresponding `pallet_membership` instances.

## API Specification

### Core Types

| Type | Description |
|------|-------------|
| `FederatedAuthorityData` | Observed membership from mainchain |
| `AuthorityMemberPublicKey` | Member's sr25519 public key (wrapped) |
| `MainchainMember` | 28-byte PolicyId identifying member on Cardano |
| `FederatedAuthorityObservationConfig` | Genesis configuration |
| `AuthBodyConfig` | Per-body configuration ([Council](../../GLOSSARY.md#council) or TC) |

### FederatedAuthorityData

```rust
pub struct FederatedAuthorityData {
    pub council_authorities: Vec<(AuthorityMemberPublicKey, MainchainMember)>,
    pub technical_committee_authorities: Vec<(AuthorityMemberPublicKey, MainchainMember)>,
    pub mc_block_hash: McBlockHash,
}
```

### AuthBodyConfig (std only)

```rust
pub struct AuthBodyConfig {
    pub address: String,                      // Cardano script address
    pub policy_id: PolicyId,                  // Native asset policy
    pub members: Vec<sr25519::Public>,        // Initial sidechain members
    pub members_mainchain: Vec<MainchainMember>, // Mainchain member hashes
}
```

### FederatedAuthorityObservationConfig

```rust
pub struct FederatedAuthorityObservationConfig {
    pub council: AuthBodyConfig,
    pub technical_committee: AuthBodyConfig,
}
```

### Runtime API

```rust
pub trait FederatedAuthorityObservationApi {
    fn get_council_address() -> MainchainAddress;
    fn get_council_policy_id() -> PolicyId;
    fn get_technical_committee_address() -> MainchainAddress;
    fn get_technical_committee_policy_id() -> PolicyId;
}
```

### Inherent

| Identifier | Type |
|------------|------|
| `faobsrve` | `FederatedAuthorityData` |

### Helper Functions (std only)

| Function | Description |
|----------|-------------|
| `ed25519_to_mainchain_member` | Convert Ed25519 pubkey to MainchainMember |

### Cargo Features

| Feature | Default | Description |
|---------|---------|-------------|
| `std` | Yes | Enables serde, sp_core for config parsing |

## Usage

### Genesis Configuration

```json
{
  "council": {
    "address": "addr_test1...",
    "policy_id": "abc123...",
    "members": ["0x..."],
    "members_mainchain": ["abc123..."]
  },
  "technical_committee": {
    "address": "addr_test1...",
    "policy_id": "def456...",
    "members": ["0x..."],
    "members_mainchain": ["def456..."]
  }
}
```

### Processing Inherent Data

```rust
use midnight_primitives_federated_authority_observation::FederatedAuthorityData;

fn process_authority_data(data: FederatedAuthorityData) {
    // Extract sr25519 public keys for membership updates
    let council_members: Vec<AccountId> = data.council_authorities
        .into_iter()
        .map(|(pk, _)| pk.0.try_into().expect("valid"))
        .collect();
}
```

## Integration

### Dependencies

- `sidechain-domain` - `MainchainAddress`, `PolicyId`, `McBlockHash`
- `sp-api` / `sp-inherents` - Runtime API and inherent support
- `sp-core` (std) - Cryptographic types

### Used By

- `pallet-federated-authority-observation` - Inherent processing
- `midnight-node` - Data source configuration
- `midnight-node-res` - [Genesis](../../GLOSSARY.md#genesis) config loading

## See Also

- [pallet-federated-authority-observation](../../pallets/federated-authority-observation/README.md)
- [pallet-federated-authority](../../pallets/federated-authority/README.md)

