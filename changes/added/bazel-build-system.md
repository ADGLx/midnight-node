#node #ci
# Add Bazel build system alongside Cargo

Bazel can now build the node binary, WASM runtime, and run clippy/fmt checks without Cargo. `crate_universe` generates BUILD files from `Cargo.toml`/`Cargo.lock`, keeping a single source of truth.

Earthly targets (`+check-rust`, `+build`, `+test`) have been migrated to use Bazel under the hood. Clippy flags are derived from `Cargo.toml` workspace lints via a script, and `rustfmt.toml` is unified across the repo.

Code fixes for global clippy compliance (redundant closures, needless borrows, `unwrap_in_result` handling) and a custom WASM toolchain for the runtime build are included.

PR: https://github.com/midnightntwrk/midnight-node/pull/915
