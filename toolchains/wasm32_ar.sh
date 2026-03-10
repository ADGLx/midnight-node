#!/bin/sh
# Wrapper for llvm-ar targeting wasm32.
exec /usr/bin/llvm-ar "$@"
