#!/bin/sh
# Wrapper that invokes clang targeting wasm32.
# Used by the Bazel CC toolchain for wasm32v1-none cross-compilation.
exec /usr/bin/clang --target=wasm32-unknown-unknown "$@"
