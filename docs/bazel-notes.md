# Bazel Build Notes (midnight-node)

Branch: `giles-bazel`

## Status

**All 21 Earthly targets build successfully.** `earthly -P +build-bazel` passes (3,015 actions).

### Building (21 targets)

util/upgrader, util/aiken-deployer, util/documented (3 sub-crates), metadata,
primitives/ledger, primitives/beefy, primitives/midnight, primitives/system-parameters,
primitives/ics-observation, primitives/reserve-observation,
primitives/cnight-observation, primitives/federated-authority-observation,
primitives/mainchain-follower,
ledger/helpers, ledger,
pallets/throttle, pallets/version, pallets/federated-authority,
pallets/system-parameters, pallets/federated-authority-observation, runtime/common

### Not yet attempted

- res — needs optional deps for `chain-spec` feature
- relay — depends on rs_merkle (git dep), pallas, env_logger
- pallets/midnight, pallets/midnight-system — depend on ledger
- pallets/cnight-observation/mock, pallets/midnight/rpc, pallets/system-parameters/rpc
- runtime — needs substrate_wasm_builder in build.rs
- node — 100+ deps, binary target
- util/toolkit — binary target

### Resolved: `parity-scale-codec-derive` + `proc-macro-crate`

**Root cause:** Substrate crates in the polkadot-sdk workspace use `workspace = true`
style deps in their `Cargo.toml` files:

```toml
[dependencies]
codec = { workspace = true }
```

In Bazel, `CARGO` env var is not set, so `proc-macro-crate` v3.3.0 can't call
`cargo locate-project --workspace` to find the workspace root. Without workspace
resolution, `workspace = true` deps can't be resolved, and `proc-macro-crate`
fails to find `parity-scale-codec` when deriving `Encode`/`Decode`.

**Fix:** Patch `proc-macro-crate` v3.3.0 via `crate.annotation()` in MODULE.bazel
to fall back to the canonical crate name (hyphens → underscores) when the
lookup fails. This is correct for Bazel because `@crates//:parity-scale-codec`
produces a crate named `parity_scale_codec` — the canonical name.

```starlark
crate.annotation(
    crate = "proc-macro-crate",
    version = "3.3.0",
    patches = ["//patches:proc-macro-crate-bazel-fallback.patch"],
    patch_args = ["-p1"],
)
```

Patch file: `patches/proc-macro-crate-bazel-fallback.patch`
- Wraps `crate_name()` with `bazel_fallback()` that reads `Cargo.toml` directly
- If Cargo.toml is unreadable (not in sandbox), falls back to canonical crate name
- If Cargo.toml IS readable but crate not found, returns `CrateNotFound` error
- The error path is critical: `frame-support-procedural` probes for `polkadot-sdk-frame`
  first (via `generate_access_from_frame_or_crate`). If the probe succeeds wrongly,
  it generates code referencing `polkadot_sdk_frame::deps::frame_support` — which
  doesn't exist. The error must propagate so the fallback tries `frame-support` directly.
- **Consequence:** any `rust_library` using `frame-support` macros (`#[pallet]`,
  `RuntimeDebugNoBound`, etc.) MUST have `compile_data = ["Cargo.toml"]` so the
  fallback can read the manifest and properly reject `polkadot-sdk-frame`.

## Key Files

- `MODULE.bazel` — bzlmod config, uses `rules_rust` with `crate.from_cargo()`
- `.bazelrc` — compiler/linker/perf settings
- `.bazelversion` — pinned to `7.6.0`
- `.bazelignore` — excludes `target`, `ui`, `local-environment`, `.direnv`
- `BUILD.bazel` (root) — just `exports_files(["Cargo.toml", "Cargo.lock"])`
- `patches/` — proc-macro-crate patch
- Per-crate `BUILD.bazel` files in ~38 directories

## Blocking Bug 1: crate_universe duplicate key in `defs.bzl`

`rules_rust` crate_universe generates `@crates//:defs.bzl` with a `_NORMAL_ALIASES`
dict that maps `Label → alias_name`. When two Cargo dependency aliases resolve to the
same underlying crate (same package name + version from different sources), the dict
gets duplicate keys and Starlark refuses to parse the file.

### Workaround

Don't use `all_crate_deps()` from `defs.bzl`. Reference crate targets directly:

```starlark
deps = ["@crates//:clap", "@crates//:tokio"]
```

The alias targets in `@crates//:BUILD.bazel` work fine — only the `defs.bzl` helper
has the duplicate key issue.

## Blocking Bug 2: `aliases` dict key collision for renamed deps

### Mechanism

rules_rust `aliases` attribute is `label_keyed_string_dict`. When the rule processes
aliases, it resolves each key through Bazel `alias()` rules to the actual spoke repo
target, then matches against `crate_info.owner` (the spoke target's label):

```python
# rustc.bzl line 271: keys are resolved Labels (through alias())
aliases = {k.label: v for k, v in aliases.items()}

# line 285: matched against the dep's owner
name = aliases.get(crate_info.owner, crate_info.name)
```

**Problem:** crate_universe deduplicates crates with the same `(package, version)` even
from different sources (crates.io vs git). When two workspace aliases map to the same
spoke target, the Starlark dict deduplicates keys — only the **last** entry survives.

### Affected pairs in this project

| Alias 1 (loses) | Alias 2 (wins) | Shared spoke target |
| ---------------- | -------------- | ------------------- |
| `coin_structure` (L7) | `coin_structure_ledger_8` (L8) | `crates__midnight-coin-structure-2.0.1` |
| `transient_crypto` (L7) | `transient_crypto_ledger_8` (L8) | `crates__midnight-transient-crypto-2.0.1` |
| `zkir` (L7) | `zkir_ledger_8` (L8) | `crates__midnight-zkir-2.1.0` |
| `mn_ledger_8` (L8) | `mn_ledger_hf` (HF) | `crates__midnight-ledger-8.0.0-rc.4` |
| `zswap_ledger_8` (L8) | `zswap_hf` (HF) | `crates__midnight-zswap-8.0.0-rc.4` |

### Workaround: bridge crates

For each collision, keep the winning alias in the `aliases` dict and create a thin
"bridge" `rust_library` that re-exports the original crate under the losing alias name:

```starlark
rust_library(
    name = "_coin_structure_bridge",
    crate_name = "coin_structure",
    srcs = ["bridges/coin_structure.rs"],  # pub use midnight_coin_structure::*;
    deps = ["@crates__midnight-coin-structure-2.0.1//:midnight_coin_structure"],
    edition = "2024",
)
```

Then depend on the bridge target instead of using aliases for the losing name.
Types from the bridge are identical to the original (`pub use` creates re-export
paths, not new types), so type compatibility is preserved across L7/L8/HF modules.

### Why not other solutions

- **`rustc_flags --extern`**: `string_list` attribute doesn't expand `$(location)`,
  so we can't reference the rlib path
- **`crate.annotation(crate_name = ...)`**: changes the crate name globally, breaks
  all other consumers
- **Separate spoke repos**: crate_universe doesn't distinguish same-version crates
  from different git sources — this is the upstream bug

### Resolved: `midnight_storage_core` version conflict (was Bug 3)

Two versions of `midnight-storage-core` exist in the dependency tree:

- **1.1.0** (from crates.io, used by L7/L8 deps)
- **1.1.0-hard-fork-test** (from git, used by HF deps)

Both have `crate_name = "midnight_storage_core"`. When both are in the same compilation
unit's dep tree, `#[derive(Storable)]` proc macros pick one version and traits from the
other become incompatible (~83 errors).

**Resolution:** Introduced a `hardfork` Cargo feature flag in `ledger/helpers/Cargo.toml`
and `ledger/Cargo.toml` that gates the `hard_fork_test` module and all HF deps. The
feature is in `default` so Cargo builds are completely unaffected. In Bazel, the feature
is excluded — `crate_features` omits `"hardfork"` — so HF deps aren't linked, avoiding
the version conflict.

Changes:

- `ledger/helpers/Cargo.toml` — 9 HF deps made `optional`, `hardfork` feature added to `default`
- `ledger/helpers/src/lib.rs` — `#[cfg(feature = "hardfork")]` on `pub mod hard_fork_test`
- `ledger/Cargo.toml` — `hardfork` feature added to `default` (HF deps already optional under `std`)
- `ledger/src/lib.rs` — `#[cfg(feature = "hardfork")]` on `pub mod hard_fork_test` and HF call
- `ledger/src/host_api/mod.rs` — `#[cfg(feature = "hardfork")]` on `pub mod ledger_hf`
- `Cargo.toml` (workspace) — `midnight-node-ledger-helpers` features: added `"hardfork"`
- Both `BUILD.bazel` files — HF deps/aliases removed, `crate_features` excludes `"hardfork"`

**Consequence:** Bazel builds compile L7 + L8 modules only (no HF). This is acceptable
since the hardfork test code is not needed for production builds.

### Resolved: `sqlx` macros need `CARGO` env var

`sqlx::query_as!` (v0.8.6) calls `cargo metadata` to find the workspace root
for `.sqlx/` offline cache. In Bazel, `CARGO` is not set.

**Resolution:** sqlx checks three dirs for cached queries, in order:
1. `SQLX_OFFLINE_DIR` (from `.env` file, NOT from env var)
2. `$CARGO_MANIFEST_DIR/.sqlx/`
3. `workspace_root()/.sqlx/` (needs `CARGO`)

Steps 1 and 2 don't need `CARGO`. Fix: create a `.env` file in the crate dir
with `SQLX_OFFLINE=true` and `SQLX_OFFLINE_DIR=.sqlx`, then include the root
`.sqlx/` files in `compile_data`:

```starlark
rust_library(
    compile_data = ["Cargo.toml", ".env", "//:sqlx_offline"],
)
```

The `//:sqlx_offline` filegroup is defined in the root BUILD.bazel:

```starlark
filegroup(
    name = "sqlx_offline",
    srcs = glob([".sqlx/**"]),
    visibility = ["//visibility:public"],
)
```

Note: `SQLX_OFFLINE_DIR=.sqlx` is relative to the Bazel execroot, which is where
the filegroup places the `.sqlx/` files.

## Earthly Target Lessons

### `.cargo/config.toml` breaks splice

`+build-prepare` creates `/.cargo/config.toml` (for `git-fetch-with-cli`).
`cargo-bazel splice` creates a temp workspace in `/tmp/` and walks up to find cargo
config files. Finding `/.cargo/config.toml` in a parent dir causes a hard error:
"A Cargo config file was found in a parent directory to the current workspace."

**Fix:** `RUN rm -f /.cargo/config.toml` before running bazel.

### Linker in sandbox

`-Clinker=clang` in `extra_rustc_flags` fails because `clang` isn't on PATH inside
Bazel's sandbox, even with `--action_env=PATH`. The default `cc` linker works fine.

**Working `.bazelrc` flags:**

```
build --action_env=CC=clang
build --action_env=CXX=clang++
build --action_env=PATH
build --linkopt=-fuse-ld=lld
build --@rules_rust//:extra_rustc_flags=-Clink-arg=-fuse-ld=lld
```

Note: no `-Clinker=clang` — let rustc use its default linker, just pass `-fuse-ld=lld`.

### lld package

The CI image (microdnf-based) needs `lld` installed:
`RUN microdnf -y install lld`

## BUILD.bazel Patterns

### Binary with lib.rs + main.rs

Cargo allows `main.rs` to `use cratename::...` from its own `lib.rs`. In Bazel,
split into `rust_library` + `rust_binary`:

```starlark
rust_library(
    name = "upgrader_lib",
    crate_name = "upgrader",
    srcs = glob(["src/**/*.rs"], exclude = ["src/main.rs"]),
    edition = "2024",
    deps = DEPS,
    visibility = ["//visibility:public"],
)

rust_binary(
    name = "upgrader",
    srcs = ["src/main.rs"],
    edition = "2024",
    deps = DEPS + [":upgrader_lib"],
)
```

### Crates with build.rs

Use `cargo_build_script`:

```starlark
load("@rules_rust//cargo:defs.bzl", "cargo_build_script")

cargo_build_script(
    name = "build_script",
    srcs = ["build.rs"],
    deps = [...],
)

rust_library(
    name = "my_crate",
    srcs = glob(["src/**/*.rs"]),
    deps = [..., ":build_script"],
)
```

### Proc macros

Use `rust_proc_macro` instead of `rust_library`.

## Cargo.toml Changes

- Added `exclude = ["target", "scripts"]` to `[workspace]` — prevents cargo-bazel
  from finding Cargo.toml files in excluded dirs
- Removed `[workspace]` from `scripts/fetch-utxo-ordering/Cargo.toml` — was causing
  "different workspaces" error during splice

## MODULE.bazel Config

- `crate.from_cargo()` only needs the root `manifests = ["//:Cargo.toml"]` —
  rules_rust 0.66.0+ walks the workspace automatically
- `git_override` for rules_rust points to HEAD but the duplicate key bug persists
- Rust toolchain pinned to `1.93.0` with `wasm32v1-none` extra target

## Workspace alias → `@crates//` target name mapping

The `@crates//` target name uses the crate's **package name**, not the workspace alias:

```toml
# In Cargo.toml workspace:
prometheus-endpoint = { package = "substrate-prometheus-endpoint", ... }
mn-ledger = { version = "=7.0.3-rc.1", package = "midnight-ledger" }
coin-structure = { version = "^2.0.0", package = "midnight-coin-structure" }
```

```starlark
# In BUILD.bazel, use the package name:
"@crates//:substrate-prometheus-endpoint"   # correct (not prometheus-endpoint)
"@crates//:midnight-ledger"                 # correct (not mn-ledger)
"@crates//:midnight-coin-structure"         # correct (not coin-structure)
```

### Local path deps

When a workspace crate depends on another workspace crate via `path = "..."`, the Bazel
target must reference it as a local label, not through `@crates//`:

```starlark
# documented depends on documented_proc_macro (proc macro) and documented_types (lib)
rust_library(
    name = "documented",
    deps = ["//util/documented/documented_types"],
    proc_macro_deps = ["//util/documented/documented_proc_macro"],
)
```

Proc macro path deps go in `proc_macro_deps`, not `deps`.

## Important Patterns

### `compile_data = ["Cargo.toml"]` required for frame-support users

Any `rust_library` that uses `frame-support` proc macros (`#[pallet]`,
`RuntimeDebugNoBound`, `#[derive_impl]`, etc.) MUST include Cargo.toml in
`compile_data`. Without it, the `proc-macro-crate` fallback can't distinguish
between actual dependencies and probe-only lookups (e.g. `polkadot-sdk-frame`),
causing code generation to reference non-existent crates.

### Version-qualified crate targets

When multiple versions of a crate exist in the workspace (e.g. `rand` 0.8.5 and
0.9.2), `@crates//:rand` is ambiguous. Use the versioned target name:
`@crates//:rand-0.8.5` or `@crates//:rand-0.9.2`. Check `Cargo.toml` for
which version each crate needs.

### Earthly CACHE for incremental Bazel builds

The `+build-bazel` target uses Earthly `CACHE` to persist Bazel's output base between
runs. Two cache mounts:

- `--id bazelisk` — Bazel binary downloaded by Bazelisk (~300MB)
- `--id bazel-output` — action cache, external repos, compiled artifacts

**Important:** CACHE contents aren't in the image layer, so `bazel-bin/` symlinks
can't be followed by `SAVE ARTIFACT`. The target copies artifacts to `/bazel-artifacts/`
during the RUN step (using `cp -L` to follow symlinks) before saving.

Cold build: ~450s. Cached rebuild (no source changes): ~105s (1 action).
Incremental rebuild (some changes): recompiles only affected actions.

## Next Steps

1. ~~Fix proc-macro-crate issue~~ DONE — patched via `crate.annotation()`
2. ~~Convert all BUILD.bazel files to explicit deps~~ DONE
3. ~~Test Tier 1 targets in Earthly~~ DONE — all 21 pass
4. ~~Fix sqlx CARGO env issue~~ DONE — `.env` file + `SQLX_OFFLINE_DIR`
5. ~~Fix `midnight_storage_core` version conflict~~ DONE — `hardfork` feature flag
6. File rules_rust issue for duplicate alias bug (Bug 1/2)
7. Handle `substrate_wasm_builder` for runtime build.rs
8. Add remaining targets (pallets/midnight, node, toolkit, etc.)
