# Federated Authority Observation Pallet

A pallet responsible for observing and propagating [federated authority](https://docs.midnight.network/learn/glossary#federated-authority) changes from the main chain to governance bodies ([Council](https://docs.midnight.network/learn/glossary#council) and [Technical Committee](https://docs.midnight.network/learn/glossary#technical-committee)).

## Overview

This pallet provides mechanisms for observing [federated authority](https://docs.midnight.network/learn/glossary#federated-authority) membership changes that originate from the main chain and automatically updating the corresponding governance body memberships on the [partner chain](https://docs.midnight.network/learn/glossary#partner-chain). It acts as a bridge between the main chain's authority decisions and the [partner chain](https://docs.midnight.network/learn/glossary#partner-chain)'s governance structures.

Membership changes are delivered via inherents from the mainchain follower data source, similar to cNIGHT observations. The pallet compares incoming authority lists against current on-chain membership and triggers updates only when differences are detected. This event-driven approach minimizes unnecessary state changes while ensuring governance bodies remain synchronized with mainchain authority decisions within the observation latency window.

## Features

- **[Inherent](https://docs.polkadot.com/polkadot-protocol/glossary/#inherent-transactions)-based Updates**: Receives [federated authority](https://docs.midnight.network/learn/glossary#federated-authority) data through inherents (unsigned transactions)
- **Dual Governance Support**: Manages both [Council](https://docs.midnight.network/learn/glossary#council) and [Technical Committee](https://docs.midnight.network/learn/glossary#technical-committee) memberships
- **Automatic Propagation**: Automatically updates membership pallets when changes are detected
- **Validation**: Ensures member lists meet size constraints and are non-empty
- **Change Detection**: Only creates inherents when actual membership changes occur

### Components

1. **[Inherent](https://docs.polkadot.com/polkadot-protocol/glossary/#inherent-transactions) Provider**: Extracts [federated authority](https://docs.midnight.network/learn/glossary#federated-authority) data from block inherents
2. **Membership Handlers**: Delegates membership updates to configurable handler types
3. **Change Detection**: Compares incoming authority lists with current state
4. **Event Emission**: Publishes events when memberships are reset

### Data Flow

```
Main Chain Authority Changes
           ↓
    Inherent Data
           ↓
  create_inherent()
           ↓
   reset_members()
           ↓
 MembershipHandlers
           ↓
Council/TC Membership Pallets
```
