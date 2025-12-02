# Utilities

Development tools and CLI utilities for working with the Midnight blockchain.

## Overview

```
+-----------------------------------------------------------------------+
|                            Utilities                                   |
+-----------------------------------------------------------------------+
| toolkit/      | Rust CLI for wallet, tx, and contract operations      |
| toolkit-js/   | JavaScript CLI for Compact contract execution         |
| upgrader/     | HTTP service for runtime upgrades                     |
| documented/   | Procedural macro for runtime doc extraction           |
+-----------------------------------------------------------------------+
```

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
- [GLOSSARY.md](../GLOSSARY.md) - Domain-specific terminology

