# ui

React-based frontend for interacting with the Midnight blockchain.

## Overview

A barebones UI based on the [Substrate front end template](https://github.com/jimmychu0807/substrate-front-end-template) that provides a web interface for:

- Connecting to Midnight nodes via WebSocket
- Account selection and balance display
- Transaction submission
- Chain state queries

## Installation

```bash
yarn install
```

## Usage

### Development Mode

Connect to a locally running node:

```bash
yarn start
```

### Production Build

```bash
yarn build
```

Open `build/index.html` in your browser.

## API Specification

### React Hooks

| Hook | Description |
|------|-------------|
| `useSubstrate()` | Access to Polkadot.js API, keyring, and blockchain |
| `useSubstrateState()` | Shorthand for read-only state access |

### useSubstrate State

```javascript
{
  socket,        // Remote provider WebSocket URL
  keyring,       // Available accounts keyring
  keyringState,  // "READY" | "ERROR"
  api,           // Polkadot.js API instance
  apiState,      // "CONNECTING" | "READY" | "ERROR"
  currentAccount,// Selected account pair
  setCurrentAccount // Function to update selection
}
```

### Components

| Component | Description |
|-----------|-------------|
| `TxButton` | Handles query and transaction requests |
| `AccountSelector` | Unified account selection with balance display |

## Configuration

Configuration is loaded in order (later overrides earlier):
1. `src/config/common.json`
2. Environment-specific JSON (`development.json`, `test.json`, `production.json`)
3. Environment variables

### Config Files

| File | Environment |
|------|-------------|
| `development.json` | `yarn start` |
| `test.json` | `yarn test` |
| `production.json` | `yarn build` |

### Environment Variables

| Variable | Config Key | Description |
|----------|------------|-------------|
| `VITE_PROVIDER_SOCKET` | `PROVIDER_SOCKET` | WebSocket endpoint |

### Specifying WebSocket Connection

Two methods:
1. Set `PROVIDER_SOCKET` in config JSON files
2. URL query parameter: `?rpc=ws://localhost:9944` (overrides config)

## Architecture

```
+------------------+     +------------------+     +------------------+
| React Components | --> | useSubstrate     | --> | Polkadot.js API  |
| (UI)             |     | Hook             |     |                  |
+------------------+     +------------------+     +------------------+
                                                          |
                                                          v
                                                  +------------------+
                                                  | Midnight Node    |
                                                  | (WebSocket RPC)  |
                                                  +------------------+
```

> **⚠️** Architecture is a simplified view of React/Polkadot.js integration. Actual component hierarchy in `src/substrate-lib/`.

### Directory Structure

```
ui/
+-- src/
|   +-- config/           # Environment configs
|   +-- substrate-lib/    # Polkadot.js integration
|   |   +-- components/   # TxButton, etc.
|   +-- AccountSelector.js
|   +-- Transfer.js       # Transaction example
+-- public/
+-- tests/
```

## Integration

### Dependencies

- `@polkadot/api` - Substrate RPC client
- `@polkadot/keyring` - Account management
- React + Vite - Frontend framework

### Used By

- Developers for local testing
- Demo and debugging purposes

## Browser Compatibility

Polkadot.js depends on `BigInt`. Update `package.json` for compatibility:

```json
{
  "browserslist": {
    "production": [
      ">0.2%",
      "not ie <= 99",
      "not android <= 4.4.4",
      "not dead",
      "not op_mini all"
    ]
  }
}
```

## Testing

```bash
yarn test
```

## See Also

- [node](../node/README.md) - Midnight node documentation
- [Polkadot.js Documentation](https://polkadot.js.org/docs/) - API reference
