# parity-scale-codec-derive and Bazel: The Alias Problem

## The Problem

`parity-scale-codec-derive` generates code with hardcoded crate paths that
break when the crate is aliased — which is near-universal in Substrate.

### How Cargo Builds Work (Fine)

Almost every Substrate crate aliases parity-scale-codec:

```toml
[dependencies.codec]
package = "parity-scale-codec"
```

The derive macro (`#[derive(Encode, Decode)]`) uses
[proc-macro-crate](https://crates.io/crates/proc-macro-crate) to discover
the alias at compile time. It reads `$CARGO_MANIFEST_DIR/Cargo.toml`, finds
the `[dependencies.codec]` entry with `package = "parity-scale-codec"`, and
generates `::codec::Encode` — matching the alias. Everything works.

### How Bazel Builds Break

In Bazel (via rules_rust + crate_universe):

1. rules_rust sets `CARGO_MANIFEST_DIR` to the crate's source directory
2. The derive macro calls `proc_macro_crate::crate_name("parity-scale-codec")`
3. proc-macro-crate tries to read `$CARGO_MANIFEST_DIR/Cargo.toml`
4. **Cargo.toml is NOT in the Bazel sandbox** — it's not declared in
   `compile_data`, only `*.rs` files are in `srcs`
5. The file read fails silently
6. The fallback returns the canonical name `parity_scale_codec`
7. The derive generates `::parity_scale_codec::Encode`
8. rustc has `--extern codec=/path/to/lib.rlib` (from the alias) but NOT
   `--extern parity_scale_codec=...`
9. **Compilation fails**: `could not find parity_scale_codec in the list of
   imported crates`

### The Chain of Pain

```
binary-merkle-tree (Cargo.toml: [dependencies.codec] package = "parity-scale-codec")
  └── #[derive(Encode)]
       └── parity-scale-codec-derive 3.7.5
            └── proc-macro-crate 3.3.0  →  crate_name("parity-scale-codec")
                 └── reads $CARGO_MANIFEST_DIR/Cargo.toml
                      └── FILE NOT IN SANDBOX  →  falls back to "parity_scale_codec"
                           └── generates ::parity_scale_codec::Encode
                                └── FAILS: only ::codec:: is available
```

### Scale of the Problem

This affects every crate that:

- Uses `#[derive(Encode)]`, `#[derive(Decode)]`, `#[derive(TypeInfo)]`, or
  similar Substrate derive macros
- AND aliases `parity-scale-codec` as `codec` (or any other name)

In the Substrate/Polkadot ecosystem, this is **nearly every crate**. The
`codec` alias convention is universal.

Other derive macros affected (all use proc-macro-crate):

| Derive macro crate | Looks up |
| -------------------------------- | ----------------------- |
| parity-scale-codec-derive 3.7.5 | parity-scale-codec |
| scale-info-derive 2.11.6 | scale-info |
| frame-support-procedural 37.0.0 | frame-support |
| sp-api-proc-macro 25.0.0 | sp-api |
| borsh-derive 1.5.7 | borsh |

## The Fix

Two-part solution:

### 1. Patch proc-macro-crate (already done)

A patch on proc-macro-crate 3.3.0 adds a Bazel fallback that reads
Cargo.toml to discover alias→package mappings. See
`patches/proc-macro-crate-bazel-fallback.patch`.

The fallback handles two cases:

- **crates.io crates** (resolved deps): Cargo.toml has
  `codec = { package = "parity-scale-codec", version = "..." }`.
  The fallback matches by `package` field.
- **git-sourced crates** (workspace deps): Cargo.toml has
  `codec = { workspace = true }` with NO package field. The workspace
  root Cargo.toml (which has the actual mapping) is stripped by
  `strip_prefix`. The fallback uses a segment-matching heuristic:
  if `dep_name` ("codec") matches any hyphen-separated segment of
  `orig_name` ("parity-scale-codec"), it's treated as the alias.

**Important:** The MODULE.bazel.lock caches the crate_universe extension
output. If you add or change a `crate.annotation(patches = ...)`, you
MUST regenerate the lockfile (delete MODULE.bazel.lock or run
`bazel mod lockfile-update`).

### 2. Make Cargo.toml Available in the Sandbox

The patch is useless if Cargo.toml isn't in the Bazel sandbox.
Add a `compile_data_glob` annotation:

```starlark
# In MODULE.bazel
crate.annotation(
    crate = "*",
    compile_data_glob = ["Cargo.toml"],
)
```

This ensures every crate's Cargo.toml is declared as a compile-time input,
making it available for proc-macro-crate to read during derive expansion.

## Alternative Approaches Considered

| Approach | Why it doesn't work |
| ----------------------------------------- | ------------------------------------------------------------ |
| Patch each consumer crate | Hundreds of crates need patching |
| Add `#[codec(crate = codec)]` everywhere | Can't modify third-party source |
| Dual `--extern` flags | Don't know .rlib paths at annotation time |
| Remove aliases from crate_universe | Source code uses `codec::` everywhere, would break |
| Set `CARGO_MANIFEST_DIR` per crate | Already set by rules_rust, that's not the issue |
| Disable sandboxing | Destroys reproducibility guarantees |

## Related

- `proc-macro-crate` also needs `CARGO` env var for workspace dep resolution,
  but external crates from crate_universe don't use workspace inheritance,
  so the Cargo.toml-only fallback is sufficient.
- proc-macro-crate 1.1.3 exists in the dep tree (used by multihash-derive)
  but is less critical — multihash doesn't alias its codec dep.
