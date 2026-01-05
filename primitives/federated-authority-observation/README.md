# midnight-primitives-federated-authority-observation

Shared types for observing [federated authority](https://docs.midnight.network/learn/glossary#federated-authority) membership from Cardano.

## Overview

This crate defines types for synchronizing governance body membership ([Council](https://docs.midnight.network/learn/glossary#council), [Technical Committee](https://docs.midnight.network/learn/glossary#technical-committee)) from Cardano to Midnight. Membership changes observed on Cardano are propagated via inherents to update the corresponding `pallet_membership` instances.

## API Specification

### Core Types

- [**`FederatedAuthorityData`**](https://github.com/midnightntwrk/midnight-node/blob/main/primitives/federated-authority-observation/src/lib.rs#L107) - Observed membership from mainchain
- [**`AuthorityMemberPublicKey`**](https://github.com/midnightntwrk/midnight-node/blob/main/primitives/federated-authority-observation/src/lib.rs#L67) - Member's sr25519 public key (wrapped)
- [**`MainchainMember`**](https://github.com/midnightntwrk/midnight-node/blob/main/primitives/federated-authority-observation/src/lib.rs#L30) - 28-byte PolicyId identifying member on Cardano
- [**`FederatedAuthorityObservationConfig`**](https://github.com/midnightntwrk/midnight-node/blob/main/primitives/federated-authority-observation/src/lib.rs#L168) - Genesis configuration
- [**`AuthBodyConfig`**](https://github.com/midnightntwrk/midnight-node/blob/main/primitives/federated-authority-observation/src/lib.rs#L152) - Per-body configuration ([Council](https://docs.midnight.network/learn/glossary#council) or TC)

### Inherent

- [**`INHERENT_IDENTIFIER`**](https://github.com/midnightntwrk/midnight-node/blob/main/primitives/federated-authority-observation/src/lib.rs#L27) - `faobsrve` - FederatedAuthorityData

### Helper Functions (std only)

- [**`ed25519_to_mainchain_member`**](https://github.com/midnightntwrk/midnight-node/blob/main/primitives/federated-authority-observation/src/lib.rs#L34) - Convert Ed25519 pubkey to MainchainMember

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

## Integration

### Dependencies

- `sidechain-domain` - `MainchainAddress`, `PolicyId`, `McBlockHash`
- `sp-api` / `sp-inherents` - Runtime API and inherent support
- `sp-core` (std) - Cryptographic types

### Used By

- `pallet-federated-authority-observation` - Inherent processing
- `midnight-node` - Data source configuration
- `midnight-node-res` - [Genesis](https://docs.polkadot.com/polkadot-protocol/glossary/#genesis-configuration) config loading

## See Also

- [pallet-federated-authority-observation](../../pallets/federated-authority-observation/README.md)
- [pallet-federated-authority](../../pallets/federated-authority/README.md)

