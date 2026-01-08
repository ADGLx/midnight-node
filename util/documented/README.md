# documented

Procedural macro for extracting documentation strings at runtime.

## Overview

This workspace provides tools for accessing Rust doc comments (`///`) at runtime. This is useful for generating help text, configuration documentation, or API descriptions from source code.

The workspace contains two crates:
- `documented_types` - Core types and traits
- `documented_proc_macro` - The `#[documented]` derive macro

## Usage

### Deriving Documentation

```rust
use documented::Documented;

/// This is a configuration option.
/// It controls the behavior of the system.
#[derive(Documented)]
struct Config {
    /// The maximum number of retries.
    max_retries: u32,
    
    /// Timeout in seconds.
    timeout: u64,
}

// Access documentation at runtime
assert_eq!(Config::DOCS, "This is a configuration option.\nIt controls the behavior of the system.");
```

### Field Documentation

```rust
// Access field documentation
let fields = Config::field_docs();
assert_eq!(fields.get("max_retries"), Some(&"The maximum number of retries."));
```

## Architecture

```
documented/
+-- Cargo.toml              # Workspace re-exports
+-- documented_types/       # Core trait definitions
|   +-- src/lib.rs
+-- documented_proc_macro/  # Derive macro implementation
    +-- src/lib.rs
```

## Integration

### Dependencies

- `syn` - Rust syntax parsing
- `quote` - Code generation

### Used By

- [`midnight-node`](https://github.com/midnightntwrk/midnight-node/blob/main/node/src/cli.rs) - Configuration documentation

## See Also

- [documented_types](documented_types/README.md) - Core types
- [documented_proc_macro](documented_proc_macro/README.md) - Macro implementation

