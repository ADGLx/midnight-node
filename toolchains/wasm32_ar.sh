#!/bin/sh
# Wrapper for llvm-ar targeting wasm32.
# Apple's Xcode doesn't ship llvm-ar, so prefer Homebrew LLVM.
if [ -x /opt/homebrew/opt/llvm/bin/llvm-ar ]; then
  exec /opt/homebrew/opt/llvm/bin/llvm-ar "$@"
elif [ -x /usr/local/opt/llvm/bin/llvm-ar ]; then
  exec /usr/local/opt/llvm/bin/llvm-ar "$@"
else
  exec llvm-ar "$@"
fi
