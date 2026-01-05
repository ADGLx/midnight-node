# documented_proc_macro

Procedural macro implementation for `#[derive(Documented)]`.

## Overview

This crate provides the `Documented` derive macro that extracts doc comments from Rust source code and makes them available at runtime.

## Usage

This crate is a proc-macro and should be used through the parent `documented` crate:

```rust
use documented::Documented;

/// My documented struct.
#[derive(Documented)]
struct MyStruct {
    /// A field with documentation.
    field: u32,
}
```

## Implementation

The macro:

1. Parses the input type with `syn`
2. Extracts `#[doc = "..."]` attributes (from `///` comments)
3. Generates a `Documented` trait implementation via `quote`

### Generated Code

For the above example, the macro generates:

```rust
impl Documented for MyStruct {
    const DOCS: &'static str = "My documented struct.";
    
    fn field_docs() -> &'static [(&'static str, &'static str)] {
        &[("field", "A field with documentation.")]
    }
}
```

## Dependencies

- `syn` - Rust syntax parsing
- `quote` - Code generation

## See Also

- [documented](../README.md) - Parent crate
- [documented_types](../documented_types/README.md) - Trait definitions

