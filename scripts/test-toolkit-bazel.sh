#!/bin/sh
set -euxo pipefail

# Run Bazel-built toolkit integration test binaries.
# Each binary is a standalone Rust test executable produced by `bazel build`.

MIDNIGHT_LEDGER_EXPERIMENTAL=1
export MIDNIGHT_LEDGER_EXPERIMENTAL

# common/mod.rs uses compile-time env!("CARGO_MANIFEST_DIR") = "util/toolkit" (relative)
# to locate test-images.docker-compose.yml. Run from / so it resolves correctly.
CARGO_MANIFEST_DIR=util/toolkit
export CARGO_MANIFEST_DIR

cd /

# trycmd resolves case paths relative to cwd with no base directory.
# The single_tx test uses .case("examples/single-tx.md"), so we need
# /examples/single-tx.md to exist. Symlink to the actual location.
ln -sfn util/toolkit/examples examples

for test_bin in /test-bins/*-test; do
    echo "=== Running $(basename "$test_bin") ==="
    "$test_bin" --test-threads=1
done
