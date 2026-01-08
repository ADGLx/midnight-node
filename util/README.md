# Utilities

Development tools and CLI utilities for working with the Midnight blockchain.

## Overview

This directory contains utility tools and libraries supporting Midnight node development and operations. The toolkit provides a feature-complete Rust CLI for wallet management, transaction generation, and contract deployment. The toolkit-js offers a JavaScript/TypeScript alternative for executing compiled Compact contracts. The upgrader service enables runtime upgrades via HTTP API, while the documented crate provides procedural macros for extracting documentation at runtime.

## Packages

### [toolkit/](toolkit/README.md)
**midnight-toolkit** - Feature-complete CLI for wallet management, transaction generation, contract deployment, and blockchain interaction.

### [toolkit-js/](toolkit-js/README.md)
**toolkit-js** - JavaScript/TypeScript CLI for executing compiled Compact contracts with witness implementations.

### [upgrader/](upgrader/README.md)
**upgrader** - HTTP service for triggering runtime upgrades via REST API, used in CI/CD and testing.

### [documented/](documented/README.md)
**documented** - Procedural macro workspace for extracting doc comments at runtime.

### [documented/documented_types/](documented/documented_types/README.md)
**documented_types** - Core `Documented` trait definitions.

### [documented/documented_proc_macro/](documented/documented_proc_macro/README.md)
**documented_proc_macro** - `#[derive(Documented)]` macro implementation.

## Package Index

| Package | Path | Description |
|---------|------|-------------|
| `midnight-toolkit` | `toolkit/` | CLI toolkit (Rust) |
| `toolkit-js` | `toolkit-js/` | CLI toolkit (JavaScript) |
| `upgrader` | `upgrader/` | Upgrade service |
| `documented` | `documented/` | Doc macro workspace |
| `documented_types` | `documented/documented_types/` | Doc types |
| `documented_proc_macro` | `documented/documented_proc_macro/` | Doc macro impl |

## See Also

- [node/](../node/README.md) - Node that these tools interact with
- [Glossary](https://docs.midnight.network/learn/glossary) - Domain-specific terminology

