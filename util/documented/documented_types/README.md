# documented_types

Core types and traits for the `documented` crate.

## Overview

This crate defines the `Documented` trait that types can implement to expose their documentation strings at runtime.

## API Specification

### Traits

```rust
pub trait Documented {
    /// The doc comment for this type.
    const DOCS: &'static str;
    
    /// Documentation for each field (if applicable).
    fn field_docs() -> &'static [(&'static str, &'static str)];
}
```

## Usage

Most users should use the derive macro from the parent `documented` crate rather than implementing this trait manually.

```rust
use documented_types::Documented;

struct MyType;

impl Documented for MyType {
    const DOCS: &'static str = "My documentation";
    
    fn field_docs() -> &'static [(&'static str, &'static str)] {
        &[]
    }
}
```

## See Also

- [documented](../README.md) - Parent crate with derive macro
- [documented_proc_macro](../documented_proc_macro/README.md) - Derive implementation

