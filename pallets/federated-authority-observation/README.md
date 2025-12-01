# Federated Authority Observation Pallet

A pallet responsible for observing and propagating [federated authority](../../GLOSSARY.md#federated-authority) changes from the main chain to governance bodies ([Council](../../GLOSSARY.md#council) and [Technical Committee](../../GLOSSARY.md#technical-committee)).

## Overview

This pallet provides mechanisms for observing [federated authority](../../GLOSSARY.md#federated-authority) membership changes that originate from the main chain and automatically updating the corresponding governance body memberships on the [partner chain](../../GLOSSARY.md#partner-chain). It acts as a bridge between the main chain's authority decisions and the [partner chain](../../GLOSSARY.md#partner-chain)'s governance structures.

## Features

- **[Inherent](../../GLOSSARY.md#inherent)-based Updates**: Receives [federated authority](../../GLOSSARY.md#federated-authority) data through inherents (unsigned transactions)
- **Dual Governance Support**: Manages both [Council](../../GLOSSARY.md#council) and [Technical Committee](../../GLOSSARY.md#technical-committee) memberships
- **Automatic Propagation**: Automatically updates membership pallets when changes are detected
- **Validation**: Ensures member lists meet size constraints and are non-empty
- **Change Detection**: Only creates inherents when actual membership changes occur

### Components

1. **[Inherent](../../GLOSSARY.md#inherent) Provider**: Extracts [federated authority](../../GLOSSARY.md#federated-authority) data from block inherents
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
