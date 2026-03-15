#!/bin/sh
# Wrapper that invokes clang targeting wasm32.
# Used by the Bazel CC toolchain for wasm32v1-none cross-compilation.
# Apple's system clang doesn't support wasm32, so prefer Homebrew LLVM.
if [ -x /opt/homebrew/opt/llvm/bin/clang ]; then
  exec /opt/homebrew/opt/llvm/bin/clang --target=wasm32-unknown-unknown "$@"
elif [ -x /usr/local/opt/llvm/bin/clang ]; then
  exec /usr/local/opt/llvm/bin/clang --target=wasm32-unknown-unknown "$@"
else
  exec clang --target=wasm32-unknown-unknown "$@"
fi
